// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Module to get information about monitors

pub mod winapi;

pub type Result<T> = core::result::Result<T, String>;

#[derive(Clone, Debug, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Rect {
    pub fn width(&self) -> f64 {
        self.right - self.left
    }
    pub fn height(&self) -> f64 {
        self.bottom - self.top
    }
}

/// Monitor struct containing data about a monitor on the system
///
/// Use [`Screen::get_monitors`] to return a `Vec<Monitor>` of all the monitors on the system
///
/// [`Screen::get_monitors`]: Screen::get_monitors
#[derive(Clone, Debug, PartialEq)]
pub struct Monitor {
    device_name: String,
    primary: bool,
    rect: Rect,
    // TODO: Work area, cross_platform
    // https://developer.apple.com/documentation/appkit/nsscreen/1388369-visibleframe
    // https://developer.gnome.org/gdk3/stable/GdkMonitor.html#gdk-monitor-get-workarea
    // https://docs.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-monitorinfo
    // Unsure about x11
    work_rect: Rect,
}

impl Monitor {
    #[allow(dead_code)]
    pub(crate) fn new(device_name: String, primary: bool, rect: Rect, work_rect: Rect) -> Self {
        Monitor {
            device_name,
            primary,
            rect,
            work_rect,
        }
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Returns true if the monitor is the primary monitor.
    /// The primary monitor has its origin at (0, 0) in virtual screen coordinates.
    pub fn is_primary(&self) -> bool {
        self.primary
    }
    /// Returns the monitor rectangle in virtual screen coordinates.
    pub fn virtual_rect(&self) -> &Rect {
        &self.rect
    }

    /// Returns the monitor working rectangle in virtual screen coordinates.
    /// The working rectangle excludes certain things like the dock and menubar on mac,
    /// and the taskbar on windows.
    pub fn virtual_work_rect(&self) -> &Rect {
        &self.work_rect
    }
    pub fn width(&self) -> f64 {
        self.rect.right - self.rect.left
    }
    pub fn height(&self) -> f64 {
        self.rect.bottom - self.rect.top
    }
}
