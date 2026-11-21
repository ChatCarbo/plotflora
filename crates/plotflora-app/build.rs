// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Intègre l'icône et les métadonnées dans l'exécutable Windows.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icon.ico")
            .set("ProductName", "PlotFlora")
            .set("FileDescription", "PlotFlora — planification de jardin")
            .set("LegalCopyright", "Copyright © 2026 ChatCarbo — GPL-3.0-or-later");
        if let Err(e) = res.compile() {
            println!("cargo:warning=icône Windows non intégrée : {e}");
        }
    }
}
