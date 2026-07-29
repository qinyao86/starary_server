fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("desktop")
            .join("icons")
            .join("icon.ico");
        res.set("CompanyName", "北京魔帧影画文化科技有限公司");
        res.set("FileDescription", "Starary Server");
        res.set("ProductName", "Starary Server");
        res.set("LegalCopyright", "Copyright © 2026 北京魔帧影画文化科技有限公司");
        res.set_icon(icon.to_string_lossy().as_ref());
        res.compile().expect("failed to embed Windows resources");
    }
}
