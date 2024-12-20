#[cfg(target_os = "android")]
#[cfg_attr(target_os = "android", ndk_glue::main())]
pub fn android_main() {
    let resource_dir = std::path::PathBuf::from(ndk_glue::native_activity().internal_data_path().to_string_lossy().to_string());

    std::env::set_current_dir(&resource_dir).unwrap();
    
    let options = doukutsu_rs::game::LaunchOptions { server_mode: false, editor: false, return_types: false, external_timer: false, resource_dir: None, usr_dir: None};

    doukutsu_rs::game::init(options).unwrap();
}
