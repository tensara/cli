fn main() {
    println!("cargo:rerun-if-env-changed=TENSARA_API_BASE_URL");
}
