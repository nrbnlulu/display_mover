use std::{
    mem,
    sync::{Arc, Mutex},
};

use log::warn;
use windows::{
    Win32::{
        Devices::Display::{
            DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
            DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME,
            DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS,
            QueryDisplayConfig,
        },
        Foundation::{GetLastError, HWND, LPARAM, RECT},
        Graphics::Gdi::{
            DEVMODEW, ENUM_CURRENT_SETTINGS, EnumDisplayMonitors, EnumDisplaySettingsW,
            GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
        },
        UI::WindowsAndMessaging::{
            GetWindowThreadProcessId, MONITORINFOF_PRIMARY, SWP_NOACTIVATE, SWP_NOZORDER,
            SetWindowPos,
        },
    },
    core::PCWSTR,
};

use crate::utils::{Monitor, Rect};
use widestring::U16CString;
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, IsIconic};

unsafe extern "system" fn monitorenumproc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprect: *mut RECT,
    _lparam: LPARAM,
) -> windows::Win32::Foundation::BOOL {
    unsafe {
        let mut info_ex = MONITORINFOEXW::default();
        info_ex.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;
        let monitor_info_ex_w_ptr = &mut info_ex as *mut MONITORINFOEXW as *mut MONITORINFO;

        // https://learn.microsoft.com/zh-cn/windows/win32/api/winuser/nf-winuser-getmonitorinfoa
        if !GetMonitorInfoW(hmonitor, monitor_info_ex_w_ptr).as_bool() {
            warn!(
                "Failed to get monitor info: {}",
                GetLastError().unwrap_err()
            );
            return windows::Win32::Foundation::FALSE;
        }
        let primary = info_ex.monitorInfo.dwFlags == MONITORINFOF_PRIMARY;
        let info = info_ex.monitorInfo;
        let rect = Rect {
            left: info_ex.monitorInfo.rcMonitor.left as f64,
            top: info_ex.monitorInfo.rcMonitor.top as f64,
            right: info_ex.monitorInfo.rcMonitor.right as f64,
            bottom: info_ex.monitorInfo.rcMonitor.bottom as f64,
        };
        let work_rect = Rect {
            left: info.rcWork.left as f64,
            top: info.rcWork.top as f64,
            right: info.rcWork.right as f64,
            bottom: info.rcWork.bottom as f64,
        };
        let sz_device = info_ex.szDevice.as_ptr();
        let mut dev_mode_w = DEVMODEW {
            dmSize: mem::size_of::<DEVMODEW>() as u16,
            ..DEVMODEW::default()
        };

        if !EnumDisplaySettingsW(PCWSTR(sz_device), ENUM_CURRENT_SETTINGS, &mut dev_mode_w)
            .as_bool()
        {
            warn!(
                "Failed to get display settings for device {}: {}",
                String::from_utf16_lossy(&info_ex.szDevice).to_string(),
                GetLastError().unwrap_err()
            );
            return windows::Win32::Foundation::FALSE;
        };
        // Convert szDevice (&[u8; 32]) to String by treating as ANSI and trimming at null terminator
        let config = get_monitor_config(info_ex).unwrap();
        let device_name = U16CString::from_vec_truncate(config.monitorFriendlyDeviceName)
            .to_string()
            .unwrap();
        let monitors = _lparam.0 as *mut Vec<Monitor>;
        (*monitors).push(Monitor::new(device_name, primary, rect, work_rect));
    }

    windows::Win32::Foundation::TRUE
}
pub(super) fn get_monitor_config(
    monitor_info_ex_w: MONITORINFOEXW,
) -> anyhow::Result<DISPLAYCONFIG_TARGET_DEVICE_NAME> {
    unsafe {
        let mut number_of_paths = 0;
        let mut number_of_modes = 0;
        GetDisplayConfigBufferSizes(
            QDC_ONLY_ACTIVE_PATHS,
            &mut number_of_paths,
            &mut number_of_modes,
        )?;

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); number_of_paths as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); number_of_modes as usize];

        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut number_of_paths,
            paths.as_mut_ptr(),
            &mut number_of_modes,
            modes.as_mut_ptr(),
            None,
        )?;

        for path in paths {
            let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                    size: mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                    adapterId: path.sourceInfo.adapterId,
                    id: path.sourceInfo.id,
                },
                ..DISPLAYCONFIG_SOURCE_DEVICE_NAME::default()
            };

            if DisplayConfigGetDeviceInfo(&mut source.header) != 0 {
                continue;
            }

            if source.viewGdiDeviceName != monitor_info_ex_w.szDevice {
                continue;
            }

            let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                    size: mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                    adapterId: path.sourceInfo.adapterId,
                    id: path.targetInfo.id,
                },
                ..DISPLAYCONFIG_TARGET_DEVICE_NAME::default()
            };

            if DisplayConfigGetDeviceInfo(&mut target.header) != 0 {
                continue;
            }

            return Ok(target);
        }

        Err(anyhow::anyhow!("Get monitor name failed"))
    }
}
pub fn get_monitors() -> Vec<Monitor> {
    let mut monitors: Vec<Monitor> = Vec::new();

    unsafe {
        if EnumDisplayMonitors(
            None,
            None,
            Some(monitorenumproc),
            LPARAM(&mut monitors as *mut Vec<Monitor> as _),
        )
        .as_bool()
        {
            warn!(
                "Failed to Enumerate Display Monitors: {}",
                GetLastError().unwrap_err()
            );
        };
        monitors
    }
}

struct EnumWindowProcPayload {
    pub target_pid: isize,
    pub windows: Mutex<Vec<HWND>>,
}
impl EnumWindowProcPayload {
    fn new(target_pid: isize) -> Self {
        Self {
            target_pid,
            windows: Mutex::new(Vec::new()),
        }
    }
}

unsafe extern "system" fn enum_windows_proc(
    hwnd: HWND,
    lparam: LPARAM,
) -> windows::Win32::Foundation::BOOL {
    let mut pid: u32 = 0;

    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    let payload = unsafe { Arc::from_raw(lparam.0 as *const EnumWindowProcPayload) };
    if pid == payload.target_pid as u32 && !unsafe { IsIconic(hwnd).as_bool() } {
        // Store the HWND in lparam
        payload.windows.lock().unwrap().push(hwnd);
    }

    windows::Win32::Foundation::TRUE
}

pub fn get_pid_hwnd(process_id: isize) -> anyhow::Result<Option<HWND>> {
    let payload = Arc::new(EnumWindowProcPayload::new(process_id));
    unsafe {
        let ptr = Arc::into_raw(payload.clone());
        EnumWindows(Some(enum_windows_proc), LPARAM(ptr as _))?;
    }
    let windows = payload.windows.lock().unwrap();
    if !windows.is_empty() {
        Ok(Some(windows[0]))
    } else {
        Ok(None)
    }
}

pub fn move_window_to_monitor(window: HWND, monitor: &Monitor) -> anyhow::Result<()> {
    let rect = monitor.virtual_rect();
    unsafe {
        SetWindowPos(
            window,
            None,
            rect.left as i32,
            rect.top as i32,
            rect.width() as i32,
            rect.height() as i32,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
    }

    Ok(())
}
