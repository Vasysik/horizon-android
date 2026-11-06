// Android native Installed Mods entry for the bundled Veinminer integration.
pub(super) fn run(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: crate::game::veinminer::ID.into(),
        title: "Veinminer 0.3.2".into(),
        description:
            "Automatically mines connected matching blocks. Experimental community integration."
                .into(),
        keys: Vec::new(),
    });
}
