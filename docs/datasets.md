# Dataset acquisition and handling

Do not automatically download restricted datasets or commit patient images, labels with identifiers, model weights, or derived personal data.

| Dataset | Original access | Terms and approval |
|---|---|---|
| IDRiD | https://idrid.grand-challenge.org/ | CC BY 4.0 per existing validation documentation; retain attribution and verify current terms before reuse. |
| APTOS 2019 | https://www.kaggle.com/competitions/aptos2019-blindness-detection | Kaggle competition rules apply; account and competition access may be required. |
| Messidor / Messidor-2 | https://www.adcis.net/en/third-party/messidor/ | Follow program partner access conditions; do not assume redistribution rights. |
| DDR | https://github.com/nkicsl/DDR-dataset | Research use only per existing validation documentation; do not redistribute images. |

Store approved local copies under `validation/<experiment>/datasets/`, excluded by `.gitignore`. Record source URL, approval date, license snapshot, archive checksum, preprocessing version, and split manifest locally. Use patient-level splits and fixed seeds. Keep grading metrics (quadratic weighted kappa, macro F1, confusion matrix, sensitivity/specificity) separate from detection (mAP) and segmentation (Dice/IoU). Confirm legal terms with dataset owners before institutional use.
