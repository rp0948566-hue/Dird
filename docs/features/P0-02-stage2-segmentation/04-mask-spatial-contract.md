# Stage II mask and spatial contract

One model invocation returns one independent artifact. It is not a DR grade,
lesion count, vessel caliber or artery/vein label. A failed invocation is an
error status; never replace it with a blank/zero mask.

Every successful result must include:

- persisted nonzero examination ID; source image SHA-256; optional laterality
  only if recorded by caller;
- model kind (`MA`, `HE`, `EX`, `CWS`, `Vessel`), exact upstream version,
  converted ONNX SHA-256 and the original-checkpoint SHA-256 in provenance;
- original image width/height and mask width/height; masks must map to original
  pixel coordinates, with source crop, resize, letterbox offsets and FOV policy
  retained in a typed transform;
- probability map or binary mask encoding, preprocessing version, threshold
  and strict/inclusive comparison operator, provider, timing, status/error.

Lesion model tensor map is RGB/NCHW `[1,3,1024,1024]` ImageNet-normalized.
MA/HE inverse map is bilinear probability resize to the detected circle crop
and paste at `(x1,y1)` in a zero full-frame canvas. EX/CWS first remove
letterbox padding, then bilinear resize to rectangular crop, then paste at its
source rectangle. Threshold/postprocessing follows upstream and happens at
crop resolution; store the restored binary mask at original frame size.

Vessel tensor map is RGB/NCHW `[1,3,512,512]`. Mask model probabilities by the
resized FOV, bilinear-resize into the square FOV crop, paste at source bounds,
then apply full-resolution FOV, threshold, 3×3 ellipse opening, and minimum
component size 30. Preserve upstream's BGR/`is_rgb=True` FOV grayscale behavior.

Use one transform enum rather than ambiguous resize ratios: `LesionCircle`
(source box + crop dimensions + stretch), `LesionLetterbox` (source box + crop
dimensions + resized dimensions + x/y pad), and `VesselFovSquare` (source box
+ crop dimensions + FOV generation/erosion policy + model resolution). Unit
tests must prove identity landmarks round-trip through each transform before
any model result is accepted.

Do not persist patient imagery or masks to unencrypted files. For now the
artifact remains an in-memory result to the application caller; durable mask
storage needs a separately designed encrypted persistence contract.
