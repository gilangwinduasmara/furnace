use os_info;

pub enum OSType {
    MacOS,
    Linux,
    Windows,
    Unknown,
}

pub fn detect_os() -> OSType {
    let info = os_info::get();
    match info.os_type() {
        os_info::Type::Macos => OSType::MacOS,
        os_info::Type::Linux => OSType::Linux,
        os_info::Type::Windows => OSType::Windows,
        _ => OSType::Unknown,
    }
}
