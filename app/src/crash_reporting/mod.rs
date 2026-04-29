use std::borrow::Cow;
use std::path::Path;

use warpui::AppContext;
use warpui::rendering::GPUDeviceInfo;

use crate::antivirus::AntivirusInfo;
use crate::auth::UserUid;

pub fn run_minidump_server(_socket_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

pub(crate) fn set_tag<'a, 'b>(_key: impl Into<Cow<'a, str>>, _value: impl Into<Cow<'b, str>>) {}

pub(crate) fn set_gpu_device_info(_gpu_device_info: GPUDeviceInfo) {}

pub fn set_antivirus_info(_antivirus_info: &AntivirusInfo) {}

pub(crate) fn init(_ctx: &mut AppContext) -> bool {
    false
}

pub fn uninit_sentry() {}

pub fn init_cocoa_sentry() {}

pub fn uninit_cocoa_sentry() {}

pub fn crash() {}

pub fn set_user_id(_user_id: UserUid, _email: Option<String>, _ctx: &mut AppContext) {}

pub fn set_client_type_tag(_client_id: &str) {}
