fn main() {
    // Google 同步客户端在编译时读入，换了要重编
    println!("cargo:rerun-if-env-changed=REICKY_GOOGLE_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=REICKY_GOOGLE_CLIENT_SECRET");
    tauri_build::build()
}
