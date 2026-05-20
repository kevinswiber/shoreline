# Input Request Model

## CLI Surface

Use `shore review input-request` for durable requests that need attention, a decision, or an
explicit response:

```bash
shore review input-request open --track human:kevin --title "Need approval" \
  --reason manual-decision-required
shore review input-request list [--status open|responded|ambiguous|all]
shore review input-request fetch <input-request-id> [--include-body]
shore review input-request respond <input-request-id> --outcome approved [--reason "approved"]
```

The older `shore review intervention` command family was removed before a stable release. Existing
local `.shore/` data from that development surface should be discarded and recaptured.

## Legacy Intervention Events

Earlier development versions of Shore wrote intervention events. Current Shore uses input request
events instead. Because Shore has not released this storage contract, the supported migration is to
discard the old local `.shore/` directory and recapture the review.
