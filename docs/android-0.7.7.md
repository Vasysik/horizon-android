# Horizon Android 0.7.7 development

This version is in development; no device-wide performance improvement is established.

- Reuse menu blur bind groups until the target size changes, instead of recreating six per frame. Filtering, resolution and pass order are unchanged.
- Skip the identity blur chain when the radius is zero.
- Adreno 304 investigation: removing terrain fade discard alone did not fix transparent terrain. Removing alpha discard as an additional private probe did not restore correct rendering either. Neither shader experiment is retained.
- The 0.7.6 owner report shows substantially higher GPU time in blurred menus. These are observational, mixed-transition samples, not a controlled benchmark.
- The Mali title-screen ANR from RC1 resolves to Renderer::draw_frame -> Queue::submit -> Mali GLES. It remains unconfirmed on the current build.
- Upstream tamed-pet owner protection (PR #4184, b80768cd4) is already included in Android 0.7.6. The other symptom from Android #31, following the owner's attack target, still needs an end-to-end test.

Turnip cannot supply Vulkan support for Adreno 304; Mesa excludes Adreno 5xx and earlier. Optional custom-driver loading for supported hardware remains a separate investigation.
