use apple_sdk::Platform as ApplePlatform;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=BINDGEN_EXTRA_CLANG_ARGS");

    let build_target = std::env::var("TARGET").expect("no target set");
    let target_platform =
        ApplePlatform::from_target_triple(&build_target).expect("unknown apple platform");
    let sdk = target_platform.filesystem_name().to_lowercase();

    let target_arg = format!("--target={}", build_target);
    let sdk_path = std::process::Command::new("xcrun")
        .args(&["--sdk", &sdk, "--show-sdk-path"])
        .output()
        .expect("could not find sdk, if you are running on mac this might be an issue")
        .stdout;
    let sdk_str = std::str::from_utf8(&sdk_path)
        .expect("invalid output from xcrun")
        .trim_end();
    println!("cargo:rustc-link-search=framework={}", sdk_str);
    println!("cargo:rustc-link-lib=framework=IOUSBHost");
    let clang_args = vec![
        "-x",
        "objective-c",
        "-fblocks",
        &target_arg,
        "-isysroot",
        sdk_str,
    ];

    let bindings = bindgen::Builder::default()
        .clang_args(&clang_args)
        .header_contents("IOUSBHost.h", "#include<IOUSBHost/IOUSBHost.h>")
        .layout_tests(false)
        .objc_extern_crate(true)
        .blocklist_item("timezone")
        .blocklist_item("IUIStepper")
        // HFS* items have conflict of packed and align repr tags
        .blocklist_item("HFSCatalogFolder")
        .blocklist_item("HFSCatalogFile")
        .blocklist_item("HFSPlusCatalogFile")
        .blocklist_item("HFSPlusCatalogFolder")
        .blocklist_type("id")
        // same with FndrOpaqueInfo
        .blocklist_item("FndrOpaqueInfo")
        .blocklist_function("dividerImageForLeftSegmentState_rightSegmentState_")
        .blocklist_item("objc_object")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("unable to generate bindings");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("env variable OUT_DIR not found"));
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("couldnt write bindings");
}
