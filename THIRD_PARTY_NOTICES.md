# Third-party notices

## wgpu-hal

`third_party/wgpu-hal` is [wgpu-hal](https://github.com/gfx-rs/wgpu) 30.0.1 by the gfx-rs
developers, with one upstream fix applied
([gfx-rs/wgpu#10124](https://github.com/gfx-rs/wgpu/pull/10124)). It is licensed under MIT or
Apache-2.0; both licenses are in that folder (`LICENSE.MIT`, `LICENSE.APACHE`).

## Create

`crates/horizon-client/src/game/kinetic_machines.rs` ports
[Create](https://github.com/Creators-of-Create/Create) 6.0.10's mechanical drill and crushing
wheels (BlockBreakingKineticBlockEntity, DrillBlockEntity, CrushingWheelBlock and
CrushingWheelControllerBlockEntity) and the recipes Create makes as the game loads
(RuntimeDataGenerator), and `processing.rs`, `basins.rs`, `fans.rs`, `belts.rs`, `saws.rs`,
`funnels.rs`, `chutes.rs`, `bearings.rs` and `arms.rs` beside it its depot, millstone,
mechanical press, basin, mechanical mixer, blaze burner, encased fan, mechanical belt,
mechanical saw, funnels, chute, bearings and mechanical arm (DepotBehaviour,
MillstoneBlockEntity, PressingBehaviour, BasinRecipe, MechanicalMixerBlockEntity,
BlazeBurnerBlockEntity, AirCurrent, FanProcessing, BeltInventory, BeltConnectorItem,
SawBlockEntity, TreeCutter, FunnelBlock, FunnelBlockEntity, BeltFunnelBlock,
BeltFunnelInteractionHandler, ChuteBlockEntity, MechanicalBearingBlockEntity,
WindmillBearingBlockEntity, BearingContraption, BlockMovementChecks, ArmBlockEntity,
ArmInteractionPoint, ArmAngleTarget and their renderers), with `horizon_client::kinetics`'s water wheel
(WaterWheelBlockEntity) and stress (KineticNetwork) and `own_states.rs`'s placement
(DirectionalAxisKineticBlock), by The Create Team, whose code is under the MIT license
(`licenses/create-MIT.txt`). Create's assets are All Rights Reserved; none are in this
repository: a shared port fetches them from Modrinth on the player's device.

## Iris Features

The shape of the horizon a shader pack's sky draws (`gfx/pack_sky.rs`) comes from
[Iris Features](https://github.com/FatulDev/Horizon-Iris-Compatibility-Mod), a native mod for
Horizon, Copyright (c) 2026 Riley-Neeb, under the MIT license (`licenses/iris-features-MIT.txt`).

## Pixelify Sans

`crates/horizon-theme/fonts/PixelifySans-*.ttf` is [Pixelify Sans](https://github.com/eifetx/Pixelify-Sans)
by The Pixelify Sans Project Authors, licensed under the SIL Open Font License 1.1
(`crates/horizon-theme/fonts/OFL-PixelifySans.txt`).

## Pixelarticons

`crates/horizon-theme/icons/pixel/*.svg` are icons from
[Pixelarticons](https://github.com/halfmage/pixelarticons) by Gerrit Halfmann, under the MIT
license (`crates/horizon-theme/icons/pixel/LICENSE-pixelarticons.txt`). `star.svg` is Horizon's own, drawn on
the same grid.

## OpenJDK for Android (fetched on the phone, not shipped)

To prepare the game files on a phone, Starling downloads OpenJDK 25.0.5 built for Android by
AngelAuraMC ([angelauramc-openjdk-build](https://github.com/AngelAuraMC/angelauramc-openjdk-build),
release `download_jre25`, checked against the SHA-256 in `crates/starling-account/src/java.rs`),
runs it for the preparing and deletes it after. OpenJDK is licensed under the GNU General Public
License version 2 with the Classpath Exception; its sources are at
[openjdk/jdk](https://github.com/openjdk/jdk), the build's scripts and patches in that
repository. Starling's launcher for it (`packaging/android/native/java_launcher.c`) is MIT.

## Bedrock's touch buttons (fetched on the phone, not shipped)

The touch controls wear Minecraft: Bedrock Edition's own buttons, which Starling downloads on the
phone from Mojang's [bedrock-samples](https://github.com/Mojang/bedrock-samples) repository (the
revision and each file's git blob SHA-1 in `crates/horizon-client/assets/bedrock-touch.json`, which
`tools/bedrock_touch_manifest.py` writes) into the player's resources, Mojang's `LICENSE.md` beside
them. They are © Mojang AB, under the Minecraft End User License Agreement the download is subject
to; Starling carries none of them. Where they are missing, Starling's own buttons
(`crates/horizon-client/assets/touch`, drawn by `packaging/touch/make.py`) are MIT.
