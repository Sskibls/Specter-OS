# PhantomKernel OS — Hardware Compatibility Matrix

This document tracks hardware targets for PhantomKernel OS, prioritizing physical privacy controls, firmware auditability, and Linux-first compatibility.

## 1. Compatibility Tiers

| Tier | Status | Definition |
| :--- | :--- | :--- |
| **Certified** | High | Physical HKS (Cam/Mic), Coreboot support, Disabled ME/PSP, Full TPM 2.0. |
| **Supported** | Medium | Physical HKS (Cam/Mic), Proprietary UEFI, TPM 2.0. |
| **Compatible** | Base | Mechanical Shutters only, Linux-certified, TPM 2.0. |

---

## 2. Hardware Matrix (2025-2026 Models)

| Model | HKS (Cam/Mic) | Wi-Fi/BT HKS | Firmware | TPM 2.0 | Rating |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Purism Librem 14** | **Circuit-Level** | **Physical** | PureBoot/Coreboot | Discrete | **Certified** |
| **Framework 13/16 (2025)** | **Physical Disconnect** | Software | Proprietary (UEFI) | Integrated | **Certified** |
| **Star Labs StarFighter** | **Removable Mod** | **Physical** | Coreboot | Integrated | **Certified** |
| **System76 Pangolin (pang16)**| **Physical Switch** | Software | Proprietary (UEFI) | fTPM (AMD) | **Supported** |
| **System76 Lemur Pro** | Software/Firmware | Software | Coreboot | PTT (Intel) | **Supported** |
| **ThinkPad T14 (AMD)** | Mechanical Shutter | Software | Proprietary (UEFI) | fTPM (AMD) | **Compatible** |
| **ThinkPad X1 Carbon** | Mechanical Shutter | Software | Proprietary (UEFI) | PTT (Intel) | **Compatible** |

### Key Definitions:
- **Circuit-Level HKS:** Physically severs the power/data lines to the sensor. Software cannot override.
- **Physical Disconnect:** A mechanical switch that breaks the circuit at the module level.
- **Mechanical Shutter:** A physical slide that blocks the lens but leaves the sensor powered and the microphone active.

---

## 3. Recommended Models for PhantomKernel OS Certification

### Primary Choice: Purism Librem 14
**Why:** It is the only model providing both "air-gap" level physical switches and a fully auditable boot chain (Heads/Coreboot) with a neutralized Intel Management Engine.
- **Persona Suitability:** `Burner`, `Anon`, `Work`.
- **PhantomKernel Integration:** Full support for PureBoot tamper-evident LED indicators.

### Performance Choice: Framework Laptop 13 (2025 Edition)
**Why:** Modular design allows for physical inspection of components. The camera/mic switches are reliable physical disconnects. Excellent TPM 2.0 and Secure Boot implementation for custom keys.
- **Persona Suitability:** `Work`, `Lab`.
- **PhantomKernel Integration:** Recommended target for "Custom Secure Boot Key" enrollment testing.

### Budget/Availability Choice: ThinkPad T14 (AMD)
**Why:** Ubiquitous and highly durable. While it lacks true HKS (shutter only), it has the best Linux driver stability and firmware update support (LVFS).
- **Persona Suitability:** `General`, `Work`.
- **PhantomKernel Integration:** Target for "Kernel-level Driver Isolation" (software-based sensor disabling).

---

## 4. Hardware Security Requirements (Core Invariants)

1. **TPM 2.0 Support:** MUST be present and enabled for measured boot (Invariant 4).
2. **Secure Boot:** MUST support User Mode / Setup Mode for custom key enrollment.
3. **Firmware:** Preference for Coreboot; MUST support UEFI 2.7+.
4. **I/O Security:** MUST support IOMMU (Intel VT-d / AMD-Vi) for DMA protection.

---

## 5. Validation Checklist for New Models

- [ ] `lsusb` / `lspci`: Device disappears entirely when HKS is toggled.
- [ ] `tpm2_pcrread`: PCR values are accessible and stable across reboots.
- [ ] `fwupdmgr get-updates`: Supported by Linux Vendor Firmware Service.
- [ ] `dmidecode`: Correctly identifies serials and UUIDs for device-binding policies.
