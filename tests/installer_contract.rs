#[test]
fn windows_installer_loads_zip_support_before_reading_archives() {
    let installer =
        std::fs::read_to_string("scripts/install.ps1").expect("read Windows installer source");
    let load = installer
        .find("Add-Type -AssemblyName System.IO.Compression.FileSystem")
        .expect("installer loads Windows PowerShell zip support");
    let open = installer
        .find("[IO.Compression.ZipFile]::OpenRead")
        .expect("installer validates zip archive layout");

    assert!(
        load < open,
        "zip support must load before archive validation"
    );
}

#[test]
fn windows_installer_selftest_uses_the_documented_powershell_runtime() {
    let justfile = std::fs::read_to_string("Justfile").expect("read Justfile");

    assert!(justfile.contains(
        "powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/install-selftest.ps1"
    ));
}
