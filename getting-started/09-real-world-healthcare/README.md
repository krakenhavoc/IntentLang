# 09 - Real-World: Healthcare

A healthcare system with electronic health records, consent management, appointment scheduling, and regulatory compliance.

## Modules

| File | Domain | Key Concepts |
|------|--------|-------------|
| `patient_records.intent` | EHR / medical records | Consent-gated access, sensitivity levels, audit trails, prescriptions |
| `appointments.intent` | Clinical scheduling | Time slots, waitlist, check-in flow, no-show tracking |

## What Makes This Real-World

### Regulatory Compliance
- **Consent-gated access** — Providers cannot view or create records without active patient consent
- **Mandatory audit trails** — Every access to a medical record is logged with a reason
- **Sensitivity classification** — Records are classified as Normal, Restricted, or HighlyRestricted
- **Provider license verification** — Only active (non-suspended, non-revoked) providers can interact with records

### Complex Domain Modeling
- **Care teams** — Multiple providers associated with a patient
- **Medication lifecycle** — Active, Discontinued, Completed states with prescriber tracking
- **Appointment state machine** — `Scheduled -> CheckedIn -> InProgress -> Completed` with branches for cancellation and no-shows
- **Waitlist management** — Priority-based queuing with slot offers

### Safety Invariants
- **Unique medical record numbers** — No two patients share an MRN
- **Active medications need active prescribers** — If a prescriber is suspended, their active prescriptions are flagged
- **No double-booking** — A time slot can have at most one active appointment
- **Cancelled appointments free slots** — Invariant ensures slot status stays consistent

## Try It

```bash
# Validate both modules
intent check patient_records.intent
intent check appointments.intent

# Full verification
intent verify patient_records.intent
intent verify appointments.intent

# Audit the consent model
intent audit patient_records.intent
intent query patient_records.intent ConsentRequired
intent query patient_records.intent obligations

# Check appointment invariants
intent query appointments.intent invariants
intent query appointments.intent NoDoubleBooking

# Coverage analysis
intent coverage patient_records.intent
intent coverage appointments.intent
```

## Design Patterns to Notice

1. **Consent as a precondition**: Instead of building consent into the implementation, it's a `requires` clause — the spec makes it structurally impossible to skip.

2. **Audit logging as a property**: `audit_logged: true` is a declarative annotation. The implementation must provide it, and the audit bridge verifies it's present.

3. **State machine invariants**: `BookedSlotHasAppointment` and `CancelledSlotFreed` ensure the slot and appointment states stay synchronized — a common source of bugs in scheduling systems.

4. **Domain-specific edge cases**: Healthcare has unique boundary conditions (deceased patients, suspended providers) that must be handled explicitly.

## Pre-compiled IR

Each module has a pre-compiled IR file (`patient_records.ir.json`, `appointments.ir.json`). Regenerate them with:

```bash
intent compile patient_records.intent > patient_records.ir.json
intent compile appointments.intent > appointments.ir.json
```
