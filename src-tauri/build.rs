fn main() {
    #[cfg(target_os = "macos")]
    build_vision_ocr();
    tauri_build::build()
}

#[cfg(target_os = "macos")]
fn build_vision_ocr() {
    use std::process::Command;

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let swift_src = "src-swift/vision_ocr.swift";
    let obj_out = format!("{out_dir}/vision_ocr.o");
    let lib_out = format!("{out_dir}/libvision_ocr.a");

    let sdk_path = Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .expect("xcrun not found")
        .stdout;
    let sdk_path = std::str::from_utf8(&sdk_path).unwrap().trim().to_string();

    // Swift stdlib/compat libs live here
    let swift_lib_dir = "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx";

    let status = Command::new("swiftc")
        .args([
            "-parse-as-library",
            "-module-name", "VisionOCR",
            "-target", "arm64-apple-macos11.0",
            "-sdk", &sdk_path,
            "-framework", "Vision",
            "-framework", "CoreImage",
            "-emit-object",
            "-o", &obj_out,
            swift_src,
        ])
        .status()
        .expect("swiftc not found — install Xcode");

    assert!(status.success(), "swiftc failed");

    let ar_status = Command::new("ar")
        .args(["rcs", &lib_out, &obj_out])
        .status()
        .expect("ar not found");
    assert!(ar_status.success(), "ar failed");

    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-search=native={swift_lib_dir}");
    println!("cargo:rustc-link-lib=static=vision_ocr");
    println!("cargo:rustc-link-lib=static=swiftCompatibility56");
    println!("cargo:rustc-link-lib=static=swiftCompatibilityPacks");
    println!("cargo:rustc-link-lib=static=swiftCompatibilityConcurrency");
    println!("cargo:rustc-link-lib=framework=Vision");
    println!("cargo:rustc-link-lib=framework=CoreImage");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rerun-if-changed=src-swift/vision_ocr.swift");
}
