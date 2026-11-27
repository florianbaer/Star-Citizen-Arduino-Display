fn main() {
    println!("cargo:rerun-if-changed=shim/shim.cpp");
    println!("cargo:rerun-if-changed=../../lib/hud_proto");
    println!("cargo:rerun-if-changed=../../lib/hud_widgets");
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file("shim/shim.cpp")
        .include("../../lib/hud_proto")
        .include("../../lib/hud_widgets")
        .warnings(true)
        .compile("hudshim");
}
