# Fork plan: ZeroClaw Raspberry Pi voice profile

## Objective

Maintain a small, reviewable fork of ZeroClaw optimized for the Raspberry Pi push-to-talk voice workflow, while keeping upstream compatibility and avoiding a broad rewrite.

The fork should make voice interaction reliable, audio-safe, and recoverable without reducing the tool surface by default.

## Questionnaire decisions

Review outcome from the direction-setting questionnaire:

| Area | Decision |
| --- | --- |
| Fork strategy | Build a Raspberry Pi voice distribution/profile, not a broad rewrite. |
| First success criterion | Voice output must never speak raw infrastructure errors. |
| Tools | Keep the full tool surface available; add guardrails, recovery, and presentation fixes. |
| Repeated tool calls | Use warning, then recovery, then safe abort. |
| Spoken errors | Give a short plain-language cause and ask for the next step. |
| Formatter | Use a context-aware TTS-safe formatter. |
| Handoff | Ask the user whether to continue locally or hand off. |
| Voice task depth | Support medium tasks with confirmation, not only dispatch. |
| Config shape | Prefer per-channel `response_policy`. |
| Upstream stance | Prefer upstreamable changes, but do not let upstreamability block local needs. |
| First patch | Implement voice-safe error mapping first. |
| Observability | Keep full local debug for voice turns. |
| STT input | Send a full voice event, not just plain text. |
| Audio UX | Use a TTS/event protocol rather than text-only outbound webhooks. |
| Long tasks | Use a background supervisor or queue. |
| Guardrails | Recovery on repeated tools, bounded iterations/time, destructive-action approval, and spoken summary before risky action. |
| Formatter location | Start in `zeroclaw-channels`. |
| Raspberry scope | Treat Raspberry Pi voice as a first-class target. |
| Release | Prefer CI cross-builds for arm64. |
| Immediate next step | Revise this plan only; no code changes yet. |

## Current problem evidence

Observed on `rpi5-homeassistant`:

- Voice input is transcribed locally by Sherpa-ONNX and sent to the ZeroClaw webhook channel.
- A long voice request about weather heartbeat/storage caused ZeroClaw to repeat `glob_search` with identical arguments.
- The runtime loop detector aborted the turn with a technical error.
- The webhook channel forwarded `⚠️ Error: Agent loop aborted...` to TTS, so the user heard raw implementation details.

Relevant upstream files inspected:

- `crates/zeroclaw-channels/src/orchestrator/mod.rs`
  - sends `format!("⚠️ Error: {e}")` to channels on generic LLM/tool-loop errors.
  - handles no-reply and channel outcomes centrally.
- `crates/zeroclaw-channels/src/webhook.rs`
  - sends outbound webhook payloads with raw `content`.
- `crates/zeroclaw-runtime/src/agent/loop_.rs`
  - loop detector aborts repeated tool calls with `Agent loop aborted by loop detector`.
- `crates/zeroclaw-config/src/schema.rs`
  - has channel config, agent loop config, and autonomy/tool settings that can support channel-specific behavior.

## Principles

1. Keep tools available by default.
   - The goal is not to make voice weaker.
   - The goal is better voice orchestration, recovery, and presentation.
2. Never speak raw infrastructure errors.
   - Voice channel output must be safe for TTS.
3. Treat voice as a high-latency, low-bandwidth UI.
   - Short confirmations, summaries, and clarifying questions beat verbose diagnostics.
4. Prefer upstreamable abstractions, without blocking local needs.
   - General channel/error/formatter primitives should be upstreamable.
   - Raspberry Pi voice can be a first-class target in this fork, with hardware-specific details isolated behind optional profile/runtime boundaries.
5. Patch in thin layers.
   - Avoid forking providers, tool implementations, or core agent loop unless necessary.

## In scope

- Channel-specific output formatting for voice/webhook.
- Audio-safe error mapping and fallback messages.
- Loop detector recovery that changes behavior instead of only aborting.
- Voice-oriented handoff semantics for implementation-heavy requests.
- Better traceability for voice turns: STT text, tool attempts, failure class, final spoken text.
- Raspberry Pi first-class target profile, docs, and release path.

## Out of scope for the first fork phase

- Rewriting ZeroClaw in another architecture.
- Removing tools from the voice channel by default.
- Hard-wiring Sherpa/Piper as mandatory ZeroClaw core dependencies.
- Building a full scheduler/weather database feature inside ZeroClaw before the voice runtime is stable.

## Proposed architecture changes

### 1. Channel response policy

Add a channel-level response policy that can be configured for webhook/voice channels.

Candidate config:

```toml
[channels.webhook.response_policy]
mode = "voice"
audio_safe = true
max_spoken_chars = 700
error_style = "natural"
allow_raw_errors = false
```

Behavior:

- Normal replies pass through a TTS-safe formatter.
- Errors are mapped to friendly spoken messages.
- Technical details remain in logs/runtime trace, not in channel output.

Likely files:

- `crates/zeroclaw-config/src/schema.rs`
- `crates/zeroclaw-channels/src/orchestrator/mod.rs`
- `crates/zeroclaw-channels/src/webhook.rs`

### 2. Error taxonomy and voice-safe mapping

Introduce a small error classification layer for channel replies.

Classes:

- `LoopDetected`
- `ToolDenied`
- `ToolUnavailable`
- `AuthError`
- `ContextOverflow`
- `Timeout`
- `ProviderError`
- `UnknownFailure`

Voice-safe examples:

- LoopDetected: `Utknąłem przy tej prośbie. Powiedz ją krócej albo przekażę ją głównemu agentowi.`
- ToolDenied: `Nie mam teraz zgody na tę akcję. Mogę przygotować plan albo poprosić o potwierdzenie.`
- ContextOverflow: `Ta rozmowa zrobiła się za długa. Skróciłem kontekst; powtórz proszę ostatnią prośbę.`

Important: logs and runtime traces should still include sanitized technical metadata for debugging.

Likely files:

- new helper module in `crates/zeroclaw-channels/src/` or `crates/zeroclaw-runtime/src/`
- `crates/zeroclaw-channels/src/orchestrator/mod.rs`

### 3. Loop recovery before abort

Current behavior aborts repeated identical tool calls. Keep the detector, but add a recoverable path for channel turns.

Desired behavior:

1. first duplicate pattern: warning to model as today,
2. next repeat: inject a tool result telling the model to change strategy or ask a clarifying question,
3. final repeat: abort internally, but channel output is voice-safe.

Do not silently disable loop detection.

Likely files:

- `crates/zeroclaw-runtime/src/agent/loop_.rs`
- `crates/zeroclaw-runtime/src/agent/loop_detector.rs`
- `crates/zeroclaw-config/src/schema.rs`

Candidate config:

```toml
[agent.loop_recovery]
enabled = true
max_recovery_prompts = 1
channel_safe_abort = true
```

### 4. Voice handoff protocol

For long implementation tasks from voice, ZeroClaw should be able to return a structured handoff rather than trying to fully implement from ambiguous dictation.

This is not a tool reduction. Tools remain available, but the agent should choose handoff when the task needs a bigger coding session.

Candidate internal outcome:

```json
{
  "kind": "handoff",
  "summary": "Add hourly weather forecast collection for Karwieńskie Błoto Drugie...",
  "questions": ["Where should the data file live?"],
  "suggested_next_agent": "main coding assistant"
}
```

Spoken output:

`Zrozumiałem. To jest większa zmiana. Mogę spróbować poprowadzić ją tutaj krok po kroku, albo przekazać ją głównemu agentowi. Co wybierasz?`

If the user chooses local execution, the voice agent may proceed with medium-depth tasks, but should summarize intent and ask for confirmation before risky or multi-step actions.

Initial implementation decision: start as a prompt convention attached to `channels.webhook.response_policy` voice/audio-safe mode. Defer a first-class structured handoff outcome until the outbound TTS/event protocol and supervisor queue are in place.

Likely files:

- `crates/zeroclaw-channels/src/orchestrator/mod.rs`
- possibly new channel outcome variant or metadata field
- docs in `docs/book/src/channels/voice.md`

### 5. TTS-safe formatter

Add a reusable formatter for voice replies.

Rules:

- strip Markdown/code fences/tables for spoken output,
- convert units and symbols: `°C`, `*C`, `C` in weather contexts -> `stopni Celsjusza`,
- normalize percentages, ranges, common punctuation, URLs and technical tokens,
- cap length and split if needed.

Likely files:

- new module in channels or runtime, e.g. `crates/zeroclaw-channels/src/audio_safe.rs`
- tests for Polish examples.

### 6. Voice observability

Add a compact voice-turn event sequence:

- inbound webhook message id / sender / thread,
- STT source text if provided by caller metadata,
- tool attempts and loop recovery events,
- final raw model reply,
- final spoken reply after formatter,
- failure class.

This makes debugging possible without reading systemd logs from multiple services.

Likely files:

- `crates/zeroclaw-channels/src/orchestrator/mod.rs`
- existing `runtime_trace` integration.

### 7. Raspberry Pi voice as a first-class target

Add a first-class Raspberry Pi voice profile while keeping hardware-specific pieces optional:

- webhook channel on `42881`, TTS callback on `42882`,
- `response_policy.mode = "voice"`,
- audio-safe errors enabled,
- conservative loop recovery,
- full local voice-turn debug,
- full voice event input schema for STT metadata,
- TTS/event protocol for received/thinking/recovering/speaking/error states,
- docs for systemd services and local STT/TTS hooks,
- release path for arm64 binaries, preferably CI cross-build.

Likely files:

- `docs/book/src/channels/voice.md`
- `docs/book/src/ops/network-deployment.md`
- sample config under `dev/` or `examples/` if the repo has a convention.
- CI/release workflow once code changes are ready.

### 8. Long-task supervisor sketch

Voice should not try to hold a full coding session inside one spoken turn. The first implementation should be a small supervisor contract, not a new scheduler rewrite:

1. **Detect**: voice policy classifies broad or multi-step implementation requests as `needs_handoff_or_confirmation`.
2. **Confirm**: the spoken reply asks whether to continue locally or hand off.
3. **Create job**: when the user asks for handoff, create a durable job record with summary, source voice event metadata, open questions, and suggested agent/profile.
4. **Acknowledge**: TTS says the job was prepared/queued only after the durable record exists.
5. **Resume**: future voice turns can ask for job status or provide answers to open questions.

Minimal job shape:

```json
{
  "kind": "voice_long_task",
  "source": "webhook_voice",
  "audio_id": "utt-42",
  "summary": "Implement hourly weather forecast collection...",
  "status": "queued",
  "suggested_agent": "main coding assistant",
  "questions": ["Where should the data file live?"],
  "created_at_ms": 1760000000000
}
```

Initial storage options, in order:

- append-only runtime trace event for design/prototype,
- local JSONL queue under the workspace state directory,
- later: existing task/supervisor backend if upstream exposes one.

Hard rule: spoken voice must not claim that background work is running unless a real queue item or external handoff was created successfully.

## Milestones

### Milestone 0 — fork hygiene

- Create fork/branch, e.g. `rpi-voice-fork`.
- Add this plan to the repo.
- Add a `CHANGELOG-rpi-voice.md` or section in `CHANGELOG-next.md`.
- Define rebase policy: keep patches small and upstreamable.

Validation:

- `git status` clean except plan/docs.
- No code behavior changes.

### Milestone 1 — voice-safe channel errors

Implement channel response policy and map raw errors to natural spoken messages.

Acceptance criteria:

- Generic LLM/tool errors no longer send `⚠️ Error: {e}` to voice webhook when `audio_safe = true`.
- Logs and runtime trace still preserve useful debug info.
- Unit tests cover loop-detector error, timeout, context overflow, and auth/provider error.

### Milestone 2 — TTS-safe formatter

Implement reusable spoken-output formatter and apply it to webhook voice replies.

Acceptance criteria:

- Markdown/code fences stripped.
- `18°C`, `18 *C`, and weather `18 C` become natural Polish for speech.
- Technical symbols do not leak into normal spoken replies.
- Formatter is configurable or bypassable for non-voice channels.

### Milestone 3 — loop recovery

Add recoverable loop handling before hard abort.

Acceptance criteria:

- Repeated identical `glob_search` gets a recovery instruction instead of immediate user-facing raw failure.
- If still repeated, runtime aborts internally and voice channel gets a friendly fallback.
- Existing loop detector safety remains intact.

### Milestone 4 — voice handoff and confirmed medium tasks

Add explicit voice handoff behavior for larger implementation tasks, while still allowing medium tasks with confirmation.

Acceptance criteria:

- For ambiguous implementation requests, voice agent summarizes understanding and asks whether to continue locally or hand off.
- For medium local tasks, the agent asks for confirmation before risky or multi-step actions.
- It can still use tools when useful.
- It does not blindly repeat searches in an empty/unclear workspace.

### Milestone 5 — Raspberry first-class profile docs

Document the Raspberry deployment, config, and release path.

Acceptance criteria:

- A fresh reader can reproduce the webhook/TTS/channel profile.
- The docs explain what stays local: STT/TTS/audio loop.
- The docs explain what ZeroClaw owns: channel orchestration, tool use, response policy, voice events, and handoff/supervisor semantics.
- The docs describe arm64 release/install flow for Raspberry Pi.

## Pressure test

### Agreement

The core bug is not too many tools; it is poor voice-channel failure presentation and recovery. Keeping tools available is compatible with better guardrails.

### Clashes

- If tools stay fully available, voice turns can still become long-running or complex.
- If recovery is too aggressive, it may hide real bugs.
- If formatter is too broad, it may corrupt technical output the user actually asked to hear.

### Blind spots

- How much of this can be upstreamed depends on upstream maintainers' appetite for voice-specific concepts.
- Channel-specific config could become scattered if not centralized.
- Voice handoff may need product design beyond code plumbing.

### Recommendation

Start with channel-safe error mapping. It is the highest-confidence first patch because it directly prevents raw technical errors from reaching TTS. Add the config shape at the same time only if it stays small. Then add the context-aware formatter and loop recovery. Defer handoff metadata and background supervisor integration until error handling is stable.

### First step

Implement Milestone 1 as a tiny patch: add per-channel voice-safe error mapping in the orchestrator path that currently sends `⚠️ Error: {e}`.

## Rollout plan for rpi5-homeassistant

1. Build patched ZeroClaw on Mac or Pi.
2. Install binary to `/usr/local/bin/zeroclaw` with backup of current `0.7.5` binary.
3. Enable `response_policy.mode = "voice"` in `/home/rafalw/.config/zeroclaw-rpi-voice/config.toml`.
4. Restart `zeroclaw-voice-channel.service`.
5. Test with forced loop-like prompt and normal weather prompt.
6. Verify TTS says natural fallback, while logs retain the technical cause.

## Open questions

- Should the profile be called `voice`, `audio`, or `rpi_voice`?
- Should `audio_safe` apply only to outbound webhook, or to all channel replies in the selected profile?
- Handoff starts as a prompt convention in voice response policy; first-class channel outcome is deferred until TTS/event protocol and supervisor queue work.
- Should runtime trace store both raw and audio-safe reply text by default?
