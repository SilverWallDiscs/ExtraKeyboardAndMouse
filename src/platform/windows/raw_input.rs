use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Mutex, OnceLock};
use tracing::{debug, info, warn};
use windows_sys::Win32::Foundation::{GetLastError, HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::{
    GetRawInputData, GetRawInputDeviceInfoW, GetRawInputDeviceList, RegisterRawInputDevices,
    HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTDEVICELIST, RAWINPUTHEADER, RIDEV_DEVNOTIFY,
    RIDEV_INPUTSINK, RIDI_DEVICENAME, RID_INPUT, RIM_TYPEHID, RIM_TYPEKEYBOARD, RIM_TYPEMOUSE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
    TranslateMessage, WNDCLASSW,
};

const WM_INPUT_DEVICE_CHANGE: u32 = 0x00FE;
const WM_INPUT: u32 = 0x00FF;
const GIDC_ARRIVAL: usize = 1;
const GIDC_REMOVAL: usize = 2;

const HID_USAGE_PAGE_GENERIC: u16 = 0x01;
const HID_USAGE_GENERIC_MOUSE: u16 = 0x02;
const HID_USAGE_GENERIC_KEYBOARD: u16 = 0x06;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeviceKind {
    Mouse,
    Keyboard,
    OtherHid,
    Unknown,
}

impl DeviceKind {
    fn from_raw_type(raw_type: u32) -> Self {
        match raw_type {
            RIM_TYPEMOUSE => Self::Mouse,
            RIM_TYPEKEYBOARD => Self::Keyboard,
            RIM_TYPEHID => Self::OtherHid,
            _ => Self::Unknown,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Mouse => "mouse",
            Self::Keyboard => "keyboard",
            Self::OtherHid => "other HID",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug)]
struct DeviceDescriptor {
    kind: DeviceKind,
    name: Arc<str>,
}

static DEVICE_CATALOG: OnceLock<Mutex<HashMap<usize, DeviceDescriptor>>> = OnceLock::new();

fn catalog() -> &'static Mutex<HashMap<usize, DeviceDescriptor>> {
    DEVICE_CATALOG.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn run() -> Result<()> {
    unsafe {
        let instance = GetModuleHandleW(null());
        if instance.is_null() {
            return Err(anyhow!("GetModuleHandleW failed: {}", GetLastError()));
        }

        let class_name: Vec<u16> = "ExtraKeyboardAndMouse.RawInput\0".encode_utf16().collect();
        let window_class = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class_name.as_ptr(),
        };

        if RegisterClassW(&window_class) == 0 {
            return Err(anyhow!("RegisterClassW failed: {}", GetLastError()));
        }

        // An invisible top-level window receives WM_INPUT. It is never shown.
        // Crucially, RIDEV_NOLEGACY is NOT set: ordinary Windows input remains intact.
        let window: HWND = CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );

        if window.is_null() {
            return Err(anyhow!("CreateWindowExW failed: {}", GetLastError()));
        }

        register_raw_input(window)?;
        refresh_device_catalog();

        info!("Raw Input capture is active.");
        info!("Press keys or move each mouse; use RUST_LOG=debug to see per-device events.");
        info!("Connect or disconnect devices to see hot-plug notifications.");
        info!("Close this console or press Ctrl+C to stop.");

        let mut message = zeroed();
        loop {
            let result = GetMessageW(&mut message, null_mut(), 0, 0);
            if result == -1 {
                return Err(anyhow!("GetMessageW failed: {}", GetLastError()));
            }
            if result == 0 {
                break;
            }

            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    Ok(())
}

unsafe fn register_raw_input(window: HWND) -> Result<()> {
    let flags = RIDEV_INPUTSINK | RIDEV_DEVNOTIFY;
    let devices = [
        RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_GENERIC_MOUSE,
            dwFlags: flags,
            hwndTarget: window,
        },
        RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_GENERIC_KEYBOARD,
            dwFlags: flags,
            hwndTarget: window,
        },
    ];

    let succeeded = RegisterRawInputDevices(
        devices.as_ptr(),
        devices.len() as u32,
        size_of::<RAWINPUTDEVICE>() as u32,
    );

    if succeeded == 0 {
        return Err(anyhow!(
            "RegisterRawInputDevices failed: {}",
            GetLastError()
        ));
    }

    Ok(())
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_INPUT => {
            handle_raw_input(lparam as HRAWINPUT);

            // RIM_INPUT (foreground input) must be passed to DefWindowProcW so
            // Windows can perform its required cleanup. RIM_INPUTSINK is handled here.
            if wparam == 0 {
                DefWindowProcW(window, message, wparam, lparam)
            } else {
                0
            }
        }
        WM_INPUT_DEVICE_CHANGE => {
            match wparam {
                GIDC_ARRIVAL => info!("A Raw Input device was connected."),
                GIDC_REMOVAL => info!("A Raw Input device was disconnected."),
                _ => debug!(change = wparam, "Raw Input device change notification"),
            }
            refresh_device_catalog();
            0
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}

unsafe fn handle_raw_input(handle: HRAWINPUT) {
    let Some(input) = read_raw_input(handle) else {
        return;
    };

    let device_handle = input.header.hDevice;
    let descriptor = describe_device(device_handle);
    let device_name = descriptor.name.as_ref();

    match input.header.dwType {
        RIM_TYPEKEYBOARD => {
            let keyboard = input.data.keyboard;
            let is_release = (keyboard.Flags & 0x0001) != 0;
            debug!(
                device = %device_name,
                vkey = keyboard.VKey,
                scan_code = keyboard.MakeCode,
                state = if is_release { "released" } else { "pressed" },
                "Keyboard event"
            );
        }
        RIM_TYPEMOUSE => {
            let mouse = input.data.mouse;
            let button_flags = mouse.Anonymous.Anonymous.usButtonFlags;
            let button_data = mouse.Anonymous.Anonymous.usButtonData as i16;
            debug!(
                device = %device_name,
                delta_x = mouse.lLastX,
                delta_y = mouse.lLastY,
                movement_flags = mouse.usFlags,
                button_flags,
                button_data,
                "Mouse event"
            );
        }
        other => {
            debug!(device = %device_name, raw_type = other, "Unclassified raw input event");
        }
    }
}

unsafe fn read_raw_input(handle: HRAWINPUT) -> Option<RAWINPUT> {
    let mut byte_count = 0u32;
    let header_size = size_of::<RAWINPUTHEADER>() as u32;

    let query_result = GetRawInputData(handle, RID_INPUT, null_mut(), &mut byte_count, header_size);

    if query_result == u32::MAX || byte_count < header_size {
        return None;
    }

    // A keyboard RAWINPUT can be smaller than the maximum-sized RAWINPUT
    // struct because the data union is smaller. Allocate enough for the full
    // Rust struct while still allowing GetRawInputData to report its true size.
    let allocation_size = (byte_count as usize).max(size_of::<RAWINPUT>());
    // u64 storage guarantees at least the alignment required by RAWINPUT.
    let word_count = (allocation_size + size_of::<u64>() - 1) / size_of::<u64>();
    let mut storage = vec![0u64; word_count];

    let read_result = GetRawInputData(
        handle,
        RID_INPUT,
        storage.as_mut_ptr().cast(),
        &mut byte_count,
        header_size,
    );

    if read_result == u32::MAX || read_result == 0 {
        return None;
    }

    Some(*(storage.as_ptr().cast::<RAWINPUT>()))
}

unsafe fn refresh_device_catalog() {
    match enumerate_devices() {
        Ok(devices) => {
            let mut mice = 0usize;
            let mut keyboards = 0usize;
            let mut next = HashMap::new();

            for (handle, raw_type, name) in devices {
                let kind = DeviceKind::from_raw_type(raw_type);
                match kind {
                    DeviceKind::Mouse => mice += 1,
                    DeviceKind::Keyboard => keyboards += 1,
                    _ => continue,
                }

                next.insert(
                    handle,
                    DeviceDescriptor {
                        kind,
                        name: Arc::<str>::from(name),
                    },
                );
            }

            if let Ok(mut current) = catalog().lock() {
                for (handle, descriptor) in &next {
                    if !current.contains_key(handle) {
                        info!(
                            kind = descriptor.kind.label(),
                            device = %descriptor.name,
                            "Raw Input endpoint detected"
                        );
                    }
                }

                for (handle, descriptor) in current.iter() {
                    if !next.contains_key(handle) {
                        info!(
                            kind = descriptor.kind.label(),
                            device = %descriptor.name,
                            "Raw Input endpoint removed"
                        );
                    }
                }

                *current = next;
            }

            info!(mice, keyboards, "Current Raw Input endpoint count");
            if mice <= 1 && keyboards <= 1 {
                info!("Single-device mode: Windows input path remains unchanged.");
            } else {
                info!("Multiple Raw Input endpoints detected; device-specific capture is available.");
            }
        }
        Err(error) => warn!(%error, "Could not enumerate Raw Input devices"),
    }
}

unsafe fn enumerate_devices() -> Result<Vec<(usize, u32, String)>> {
    let entry_size = size_of::<RAWINPUTDEVICELIST>() as u32;
    let mut count = 0u32;

    let query_result = GetRawInputDeviceList(null_mut(), &mut count, entry_size);
    if query_result == u32::MAX {
        return Err(anyhow!(
            "GetRawInputDeviceList(size query) failed: {}",
            GetLastError()
        ));
    }

    if count == 0 {
        return Ok(Vec::new());
    }

    let mut entries = vec![zeroed::<RAWINPUTDEVICELIST>(); count as usize];
    let read_count = GetRawInputDeviceList(entries.as_mut_ptr(), &mut count, entry_size);
    if read_count == u32::MAX {
        return Err(anyhow!(
            "GetRawInputDeviceList(read) failed: {}",
            GetLastError()
        ));
    }
    entries.truncate(read_count as usize);

    let mut devices = Vec::with_capacity(entries.len());
    for entry in entries {
        let kind = DeviceKind::from_raw_type(entry.dwType);
        if !matches!(kind, DeviceKind::Mouse | DeviceKind::Keyboard) {
            continue;
        }

        let name = get_device_name(entry.hDevice)
            .with_context(|| format!("Could not read the name of a {} device", kind.label()))?;

        devices.push((entry.hDevice as usize, entry.dwType, name));
    }

    Ok(devices)
}

unsafe fn get_device_name(device: HANDLE) -> Result<String> {
    let mut character_count = 0u32;
    let size_result =
        GetRawInputDeviceInfoW(device, RIDI_DEVICENAME, null_mut(), &mut character_count);

    if size_result != 0 || character_count == 0 {
        return Err(anyhow!(
            "GetRawInputDeviceInfoW(size query) failed: {}",
            GetLastError()
        ));
    }

    let mut buffer = vec![0u16; character_count as usize];
    let read_result = GetRawInputDeviceInfoW(
        device,
        RIDI_DEVICENAME,
        buffer.as_mut_ptr().cast(),
        &mut character_count,
    );

    if read_result == u32::MAX {
        return Err(anyhow!(
            "GetRawInputDeviceInfoW(read) failed: {}",
            GetLastError()
        ));
    }

    let usable_length = (read_result as usize).min(buffer.len());
    let name = String::from_utf16_lossy(&buffer[..usable_length]);
    Ok(name.trim_end_matches('\0').to_owned())
}

unsafe fn describe_device(device: HANDLE) -> DeviceDescriptor {
    let key = device as usize;

    if let Ok(current) = catalog().lock() {
        if let Some(descriptor) = current.get(&key) {
            return descriptor.clone();
        }
    }

    let name =
        get_device_name(device).unwrap_or_else(|_| format!("Unknown Raw Input device 0x{key:X}"));
    let descriptor = DeviceDescriptor {
        kind: DeviceKind::Unknown,
        name: Arc::<str>::from(name),
    };

    if let Ok(mut current) = catalog().lock() {
        current.insert(key, descriptor.clone());
    }

    descriptor
}
