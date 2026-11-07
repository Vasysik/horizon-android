<p align="center">
  <img src="https://img.shields.io/badge/Minecraft-Java%2026.3-5D9C3F" alt="Minecraft Java 26.3">
  <img src="https://img.shields.io/badge/Rust-stable-B7410E?logo=rust&logoColor=white" alt="Rust">
  <a href="https://github.com/ludovic111/horizon"><img src="https://img.shields.io/badge/fork%20of-Horizon-F0795A" alt="Fork of Horizon"></a>
  <img src="https://img.shields.io/badge/platform-Android-2A2470" alt="Android">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-3B2F7E" alt="MIT license"></a>
</p>

<p align="center">
  <a href="#download"><b>Download</b></a> ·
  <a href="#android-status"><b>Android status</b></a> ·
  <a href="roadmap/starling.md"><b>Roadmap</b></a> ·
  <a href="#mods"><b>Mods</b></a> ·
  <a href="#build-from-source"><b>Build from source</b></a> ·
  <a href="CONTRIBUTING.md"><b>Contributing</b></a> ·
  <a href="https://github.com/ludovic111/horizon"><b>Horizon</b></a>
</p>

# Starling

**Starling is a lightweight port of Minecraft: Java Edition 26.3 to Android.** It is a fork of [Horizon](https://github.com/ludovic111/horizon), the Java game rewritten in Rust: it reads the same worlds, follows the same rules and generates the same terrain as Java Edition, and is checked against it by differential harnesses. You play it on the files from your own copy of Minecraft; it is free, open source and unofficial. And it runs natively, with no Java virtual machine and no OpenGL translation in between, so it starts fast and plays smoothly on a phone.

> Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft. Starling contains no Minecraft code, assets or data: you need your own legally obtained copy of Minecraft: Java Edition 26.3.

## Download

**[Starling v0.2.2](https://github.com/JuanLuci/starling/releases/latest)**. Already have Starling? Tap **Update** inside the game. New here? Download `starling-0.2.2-arm64-v8a.apk` from [the releases page](https://github.com/JuanLuci/starling/releases) and open it on the phone (Android asks you to allow installing apps from your browser or file manager).

**You need** a 64-bit phone with Android 8 or newer and Vulkan, and about 2 GB free. Sign in with a Microsoft account that owns Minecraft: Java Edition, or play offline (most servers need the sign-in). No computer: on the first start Starling downloads the game's files from Mojang's servers and prepares them on the phone, once (79 seconds on a POCO F6 Pro, longer on slower phones). Go fishing or change the settings while it works.

Development happens in [starling-client/starling](https://github.com/starling-client/starling), on the `starling` branch, where Starling and Horizon Android are becoming one app. Builds of every push there are **Starling Preview**, in [GitHub Actions](https://github.com/starling-client/starling/actions/workflows/android.yml) (the `starling-apk` artifact of a run; GitHub asks you to sign in to download it): an app of its own, installed beside the release, with its own `StarlingPreview` folder and sign-in. You can also [build it from source](#build-from-source).

## What works today

- **Setting up on the phone:** sign in with your Microsoft account, and the game's files come from Mojang and are prepared on the phone, by a Java runtime fetched just for that and removed after, with every language the game has. While it works, fish in a little pond, or change the settings in a hand-drawn skin made for this screen (the game's own look is part of what is being prepared). Resources prepared on a computer still open too.
- **The game:** singleplayer with vanilla's world generation, mobs, villagers, raids, redstone, every command, the HUD and the recipe book, on saves in vanilla's format, saved as soon as the app leaves the screen. Worlds from 1.8 to 26.2 are upgraded as vanilla does. Starling follows Horizon closely: v0.2.0 has its main of 2026-11-02.
- **Touch controls:** every action on screen, from the movement stick to the hotbar, chat, commands and F3, with a layout you can move, resize and fade in the game, before a world too, and Sneak as Hold or Toggle. Bedrock's control modes: a stick (floating or fixed) or a D-pad, and tap to interact (a tap hits the mob or uses the block under the finger, a press breaks the block, interacts with the mob or uses the item in the air) or aim crosshair. F3 held offers its combinations (chunk borders, hitboxes, tooltips...); Pick Block and the player list are buttons to turn on; the phone buzzes when a block breaks or a stack splits, and it can aim by turning (gyroscope, off by default). They wear Bedrock's own buttons, fetched on the phone from Mojang's samples repository after the preparing (Starling ships none of them, only their names and hashes), else Starling's own.
- **Custom buttons:** tap **Add** in **Touch Controls**, or go to **Options > Controls > Custom Buttons**. Each one holds or toggles a key, a native mod's too, or sends a command or a chat line, and wears an icon or your own image, drawn on Bedrock's plate or imported as a PNG, with another for while it is pressed.
- **Bedrock Inventory:** the creative inventory shows the items and your inventory side by side, with six rows of items and a 2x2 crafting grid, and a tap takes a full stack. It is on by default; turn it off in **Options > Controls** for the Java layout.
- **Your name and your skin:** in Options > Skin Customization, the name your worlds use and a skin from your files (or `Starling/skins`), classic or slim; a skin PNG opened with Starling is put on at once.
- **The phone's own keyboard:** a tap on a text field brings it up and a tap beside it puts it away, on every screen. Its suggestions, autocorrect, swipe typing and clipboard in the chat, signs and books (commands and searches go without autocorrect), a Paste button on the chat bar, and Enter finishing a command when only one completion fits (`/gamemode c` runs `/gamemode creative`).
- **Your files within reach:** worlds, resource packs, shader packs, mods and screenshots live in a `Starling` folder on the phone's storage, which any file manager opens (turned down on the first start, it is offered again by the Open Pack Folder buttons). Open a `.starling`, `.hmod` or `.hnmod` file with Starling to add a pack, a world or a mod.
- **Made for phones:** settings picked for your device on the first start, and a stats overlay for FPS, memory and temperature.
- **Horizon's own extras:** new menus (vanilla's are one setting away), graphics presets and more, all off by default where they would change the vanilla look. See [Settings](https://playhorizon.dev/docs/settings).
- **Resource packs and data packs:** resource packs work, and data packs load functions, predicates and item modifiers.
- **Servers:** offline-mode servers, behind a proxy such as Velocity too, with their resource packs, coloured chat, boss bars, signs, workstation menus (enchanting, anvil, smithing...), flight and the attributes they set. Plugin servers' dialogs, chat buttons, custom armour (ItemsAdder, Nexo), models riding their mobs (ModelEngine, BetterModel) and clickable furniture work, and the touch stick moves in a way each server's anticheat (GrimAC and the like) accepts: Free Direction, D-pad or Analog, remembered per server.
- **Shader packs:** Iris and OptiFine packs such as Complementary, MakeUp, BSL, Photon and Bliss run on the phone, translated when they load: put them in `Starling/shaderpacks` and pick one in **Video Settings > Shader Packs**, where the **Auto** profile fits it to the phone, beside **Shader Options**, **Resolution** and **FPS Limit**. On Auto on a Snapdragon 8 Gen 2: MakeUp 87 fps, Complementary Reimagined 59, BSL 54, Complementary Unbound 39, Photon 27 to 39. See the [roadmap](roadmap/starling.md).
- **Starling Visuals:** Starling's own shader pack, made for phones, which comes with the app: pick it in **Video Settings > Shader Packs** ([packaging/shaderpacks/starling-visuals](packaging/shaderpacks/starling-visuals), also `StarlingVisuals.zip` on the releases page). Sunlight and shadows, water with waves and depth, haze thicker down low and at dawn, sun rays through the trees and clouds, bloom, and light from a torch in your hand.
- **Mods:** Horizon's native mods (`.hnmod`), compiled from the phone itself, and content mods (`.hmod`). See [Mods](#mods).

## Android status

What the Android port covers so far; what is being done now and next is in the [roadmap](roadmap/starling.md). The game itself is Horizon's: its parity audit, measured on the desktop, is in [Horizon's project status](https://github.com/ludovic111/horizon#project-status) ([as of Starling's base](docs/horizon-parity.md)).

| Part | Progress | Notes |
| --- | --- | --- |
| The game, singleplayer | <img src="docs/media/progress/100.svg" alt="100%"> | Horizon's game, as on the desktop |
| Touch controls | <img src="docs/media/progress/100.svg" alt="100%"> | Every action, with an editable layout and buttons of your own; a stick, a D-pad or Free Direction on servers; Bedrock's buttons, fetched from Mojang |
| Starling folder, `.starling`, `.hmod` and `.hnmod` files | <img src="docs/media/progress/100.svg" alt="100%"> | Packs, worlds, mods and game files, opened with Starling |
| Resource packs | <img src="docs/media/progress/100.svg" alt="100%"> | Server packs too, loaded without freezing the game |
| Performance tuning | <img src="docs/media/progress/95.svg" alt="95%"> | Settings picked per phone, resolution down to 25% for weaker ones; mid-range phones not measured yet |
| Settings | <img src="docs/media/progress/100.svg" alt="100%"> | Options that do nothing are greyed out, those a phone can't use are hidden |
| Mods | <img src="docs/media/progress/95.svg" alt="95%"> | Native mods compiled from the phone, tried end to end with test mods; content mods; a run over the registry's mods still to do |
| Servers | <img src="docs/media/progress/100.svg" alt="100%"> | Offline-mode servers and proxies, dialogs, chat buttons and links, secure chat with a signed-in account |
| Server plugins | <img src="docs/media/progress/100.svg" alt="100%"> | Custom armour from any pack, animated models riding mobs, block and item displays, clickable furniture, item cooldowns, HUD text without shadows |
| Movement on anticheat servers | <img src="docs/media/progress/95.svg" alt="95%"> | Free Direction, D-pad or Analog per server, no pull-backs on GrimAC or Pika in tests; breaking blocks as vanilla predicts them (a dig before the move into its space); some old-version servers behind ViaVersion still pull back |
| Shader packs | <img src="docs/media/progress/95.svg" alt="95%"> | Complementary, MakeUp, BSL, Photon and Bliss run on a phone, with an Auto profile and Shader Options; some effects still differ |
| APK builds and updates | <img src="docs/media/progress/100.svg" alt="100%"> | Signed releases on the releases page, offered and installed by the game itself from v0.1.0.1 (the first offered: v0.1.1); every push also builds an APK in GitHub Actions |
| Open to LAN and invites | <img src="docs/media/progress/95.svg" alt="95%"> | Horizon's, in the game; little tried on a phone so far |
| Physical controllers | <img src="docs/media/progress/100.svg" alt="100%"> | Android's gamepads, tried on a phone |
| Setting up on the phone | <img src="docs/media/progress/100.svg" alt="100%"> | Prepared on a phone in 79 s from the official 26.3 files, with Java fetched for it; fishing and hand-drawn settings while it works |
| Microsoft account sign-in | <img src="docs/media/progress/100.svg" alt="100%"> | On the first start and in the game (the title screen's corner); the code and the ownership check work on a phone |

## Mods

Starling runs Horizon's mods. Open the file with Starling, from your file manager or your downloads, and it shows up in **Options > Mods**.

- **`.hnmod`, native mods:** tap **Compile**. Starling tells you the risks, builds the game with your mods while you fish or mine, and restarts with them. It takes a few minutes and needs a connection, and after an update you compile again. A native mod runs as part of the game, so install only the ones you trust. See [native mods](docs/native-mods.md) and [how Starling builds them](docs/starling-native-mods.md); [wrench](native-mods/wrench) is an example.
- **`.hmod`, content mods:** data-only mods, with nothing to compile. See [mods](docs/mods.md).

The old Starling mods and the WebAssembly mod format of v0.1 are gone.

## Build from source

For developers. Requirements: Rust 1.90 or newer with the `aarch64-linux-android` target, [cargo-ndk](https://github.com/bbqsrc/cargo-ndk), the Android SDK (build-tools 35.0.0, platform 36, NDK r28c), a JDK 25 and Python 3.10 or newer.

```sh
git clone -b starling https://github.com/starling-client/starling.git
cd starling
python packaging/android/build_apk.py            # the APK lands in target/android/
```

The setup's hand-drawn skin is made by `python packaging/lowbudget/make.py` (with Pillow) into `packaging/lowbudget/pack`, which the APK carries as it is.

The APK contains no game files. On the phone, Starling downloads them from Mojang's servers and prepares them there. To prepare them on a computer instead, it builds its resources from **your own copy** of Minecraft: Java Edition 26.3, verified against the checksums in `research-lock.json`, the same way Horizon does:

1. **Client:** install Java Edition with the official launcher and play 26.3 once. Horizon reads `versions/26.3/`, `libraries/` and `assets/` from the launcher's game directory. If yours lives elsewhere, pass `--minecraft-dir` or set `HORIZON_MINECRAFT_DIR`.
2. **Server jar:** the 26.3 server jar, from the `downloads.server.url` in your launcher's `versions/26.3/26.3.json`. Pass it with `--server-jar` or set `HORIZON_SERVER_JAR`.

```sh
cargo run --release -p horizon-prepare -- --prepare --server-jar ~/Downloads/server.jar   # needs JDK 25
```

The prepared files land in the gitignored `.local/horizon-client/26.3`. Zip that folder, rename the zip to `.starling` and open it with Starling. It is derived from the game: never commit it or share it.

## Contributing

Issues and pull requests are welcome: see [CONTRIBUTING.md](CONTRIBUTING.md). Changes to the game itself belong upstream, in [Horizon](https://github.com/ludovic111/horizon); Starling is the Android app around it.

## Licence

[MIT](LICENSE), for the project's original work only, as Horizon's. Dependencies keep their own licences. Minecraft is a trademark of Mojang Synergies AB; Minecraft code and assets belong to Mojang and Microsoft and are covered by the [Minecraft EULA](https://www.minecraft.net/en-us/eula). Prepared resources stay on your devices and are never published.

Third-party credits and licenses: [Third-party notices](THIRD_PARTY_NOTICES.md).

## Author

Starling, the Android port, by JuanLuci ([@JuanLuci](https://github.com/JuanLuci)). Horizon made by [Ludovic Marie](https://lu4ovic.com) ([@ludovic111](https://github.com/ludovic111)).

Tested on a POCO F6 Pro (Snapdragon 8 Gen 2, Adreno 740, 12 GB of RAM, Android 16). It has also run on a Tecno Pova 7, on Mali-G57 and Mali-G68 GPUs, on a Redmi Note 8 (Mali-G52) and on many others.
