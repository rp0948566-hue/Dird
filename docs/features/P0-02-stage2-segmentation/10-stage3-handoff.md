# Stage III handoff boundary

Stage II must eventually provide five distinct image-aligned artifacts in
stable order: MA, HE, EX, CWS and Vessel. Each artifact needs source image
SHA-256, examination identity, model identity/version/hash, original and mask
dimensions, transform, probability/binary semantics and status/error.

The Stage III LG-DRG model expects RGB plus five mask channels according to
the upstream v1.0 cascade. Stage III is not implemented here. Do not build,
load, infer, label or claim a DR grade until all five authentic Stage II masks,
their order/registration and their authorized-image tests are accepted.
