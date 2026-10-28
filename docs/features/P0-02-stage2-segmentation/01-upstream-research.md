# Upstream Stage II research

## Provenance

- Official source: [HierarchiRetina v1.0](https://github.com/asifuddin01/HierarchiRetina/tree/v1.0), tag commit `bf84b1d3c7409d2575dff84c7575c67577493a30`.
- The local source checkout is commit `27f49f5ccc1d6297903dd651cef8a660eecc48ce`; its `hierarchiretina/stage2` tree compares identical to v1.0. The later v1.0.1 tag is `69d76ae101a189b159aae1bb1db60a9d656b7bd2`.
- Upstream `scripts/weights_manifest.json` SHA-256: `af5c0451c155a4bd59f833f8e8c7c7949eb45b62beb1e022fd79057c3c920690`.
- Official release lists the exact five Stage II filenames, byte counts and weight hashes in [release v1.0](https://github.com/asifuddin01/HierarchiRetina/releases/tag/v1.0). Code license is MIT; trained weights/predictions are CC BY-NC 4.0. Each dataset retains separate terms.

## Lesion models: HSMoE-AUNet

Source: `hierarchiretina/stage2/{hsmoe_aunet.py,lesion_data.py,lesion_inference.py}`.
All four take normalized RGB float32 NCHW `[1,3,1024,1024]`. Main output is
one-channel full-resolution logits `[1,1,1024,1024]`; upstream `forward`
returns `(main_logits, aux3, aux2, moe_aux_loss)`, so inference/export must
select only element 0. Apply sigmoid, then the per-model threshold and
inverse geometry. ImageNet mean `(0.485,0.456,0.406)`, std
`(0.229,0.224,0.225)`.

- MA: base experts; circle crop then stretch; one forward; strict `prob > 0.01`.
- HE: base plus multiscale expert; circle crop then stretch; one forward; strict `prob > 0.20`.
- EX: base, multiscale, bright-spot and edge-enhancing experts; rectangle crop then aspect-preserving letterbox; average sigmoid probabilities from identity, horizontal flip and vertical flip; strict `prob > 0.325`.
- CWS: base, multiscale, bright-spot, soft-region and texture experts; rectangle crop and letterbox; same three-view average; strict `prob > 0.40`, then 5×5 elliptical closing and removal of 8-connected components smaller than `40 * max(1, crop_area / 1024²)` pixels.

MA/HE crop detection uses grayscale threshold 20, 15×15 ellipse close×3/open×2,
largest external contour, minimum enclosing circle and clipped square crop;
fallback is full frame when circle radius is at most 10% of the smaller image
dimension. EX/CWS uses threshold 15, 11×11 ellipse close×3/open×2, largest
contour bounds plus 10-pixel padding. Letterbox resize uses bilinear
interpolation, floor-rounded resized dimensions and black padding. At output,
probability maps return to crop dimensions with bilinear resize (excluding
letterbox padding), then paste into a zero-valued original-frame canvas.

## Vessel model: SwinHRUNet++

Source: `hierarchiretina/stage2/{swinhrunetpp.py,vessel_data.py,vessel_inference.py,vessel_metrics.py}`.
Input is normalized RGB float32 NCHW `[1,3,512,512]`; evaluation output is
one main logit map `[1,1,512,512]`. Architecture uses SwinV2-Base window 8,
high-resolution CNN branch, attention-gated decoder, vessel refinement and
topology refinement. Apply sigmoid and four-view identity/H/V/H+V TTA average.

FOV processing: threshold 12; 25×25 ellipse close/open; fill largest external
contour (all-FOV fallback); erode with 9×9 ellipse. The source intentionally
passes a BGR image to FOV conversion with `is_rgb=True`; preserve this channel
choice. Convert BGR→RGB, apply LAB-L CLAHE (clip 2.0, 8×8), blend equalized
green at strength 0.35, crop square FOV bounds with 1% margin, resize image
bilinear/FOV nearest to 512, then ImageNet-normalize. At output, mask
probabilities by model-resolution FOV, bilinear-resize to crop, paste into
original-frame canvas, threshold `prob >= 0.46`, restrict to full FOV, open with
3×3 ellipse, and remove 8-connected components smaller than 30 pixels.

These behaviors come from the pinned source, not Stage I preprocessing. Keep
all source crop/FOV/letterbox fields in the inverse transform. No learned
checkpoint or output has yet been executed.
