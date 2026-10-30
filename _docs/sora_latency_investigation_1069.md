# Sora latency investigation (#1069)

Investigation-only instrumentation for
[storytold/artcraft#1069](https://github.com/storytold/artcraft/issues/1069):
users reporting longer Sora times after upgrading to Basic or Pro.

**No product behavior change.** Sora models remain `is_disabled` until a
separate fix PR is approved after reviewing these metrics.

## Metrics (Datadog)

| Metric                         | Type         | When                                      |
|--------------------------------|--------------|-------------------------------------------|
| `artcraft.sora.enqueue`        | count        | Successful Fal/DB enqueue for Sora jobs   |
| `artcraft.sora.job_duration_ms`| distribution | Fal webhook success/failure for Sora jobs |

### Tags (low cardinality)

| Tag           | Values                                                              |
|---------------|---------------------------------------------------------------------|
| `model`       | `sora_2`, `sora_2_pro`                                              |
| `plan`        | `free`, `artcraft_basic`, `artcraft_pro`, `artcraft_max`, `unknown` |
| `resolution`  | `720p`, `1080p`, `other`                                            |
| `duration_s`  | `4`, `8`, `12`, `other`                                             |
| `outcome`     | `success`, `failure` (duration metric only)                         |

`plan` comes from the user's Artcraft subscription row (`free` when none).
Completion events may use `resolution:other` / `duration_s:other` when those
fields are not cheaply available on the Fal job row.

## Suggested Datadog queries

Compare paid vs free wall-clock:

```text
p50:artcraft.sora.job_duration_ms{outcome:success} by {plan,model}
p90:artcraft.sora.job_duration_ms{outcome:success} by {plan,model}
```

Enqueue volume after upgrades (are paid users submitting more?):

```text
sum:artcraft.sora.enqueue{*}.as_count() by {plan,model}
```

Filter to a single tier:

```text
p90:artcraft.sora.job_duration_ms{plan:artcraft_basic,outcome:success} by {model,resolution,duration_s}
p90:artcraft.sora.job_duration_ms{plan:free,outcome:success} by {model,resolution,duration_s}
```

## Code touchpoints

- Tag helpers / recorders:
  `crates/service/web/storyteller_web/src/http_server/endpoints/omni_gen/shared_utils/sora_latency_metrics.rs`
- Omni enqueue: `shared_video_generation.rs` (Fal branch)
- Legacy multi-function enqueue: `sora_2_*_multi_function_video_gen_handler.rs`
- Completion: Fal webhook success/failure handlers

## Follow-up

1. Deploy with `ENABLE_METRICS=true` and a Datadog API key.
2. Collect enough Sora samples (may require temporary staging enablement while
   catalog `is_disabled` remains true in production).
3. Summarize p50/p90 by `plan` for an admin.
4. Open a **separate** fix PR only after admin approval.
