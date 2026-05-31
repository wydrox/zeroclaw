# Voice & Telephony

Real-time voice input and output. Four channels cover the matrix: inbound calls, outbound speech synthesis, local microphone wake, and SIP-grade real-time conversation.

## ClawdTalk (real-time SIP)

```toml
[channels.clawdtalk]
enabled = true
telnyx_api_key = "..."
connection_id = "..."              # Telnyx SIP connection
model_provider = "voice-brain"     # references [providers.models.voice-brain]
voice = "Polly.Joanna-Neural"      # Telnyx voice ID
```

Full-duplex SIP voice powered by Telnyx. The agent talks over a real phone call — inbound or outbound. Supports barge-in, mid-turn tool use, and regional number provisioning.

**Pair with:** a `telnyx` provider for the brain (`crates/zeroclaw-providers/src/telnyx.rs`) and ensure your Telnyx account has a SIP connection with the correct webhook URL pointed at the ZeroClaw gateway.

## Voice Call (Twilio / Telnyx / Plivo)

```toml
[channels.voice_call]
enabled = true
carrier = "twilio"                 # or "telnyx", "plivo"
account_sid = "..."
auth_token = "..."
from_number = "+14155550123"
stt_provider = "whisper"
tts_provider = "elevenlabs"
```

Traditional carrier voice — the agent picks up, transcribes with STT, replies with TTS. Higher latency than ClawdTalk but works with any regular phone number and doesn't require SIP trunk provisioning.

## Voice Wake (local wake-word)

```toml
[channels.voice_wake]
enabled = true
wake_phrase = "hey claw"
engine = "porcupine"               # or "openwakeword"
model_path = "~/.zeroclaw/wake/hey-claw.ppn"
audio_device = "default"
```

Runs locally, listens on the mic, triggers agent interaction when it hears the wake phrase. Useful for:

- Physical voice assistants on SBCs
- Desktop "hotword → ask" workflows
- Always-listening home-automation agents

The agent doesn't send audio anywhere — wake detection is local. Only post-wake speech goes through STT and reaches the LLM.

## Raspberry Pi push-to-talk profile (experimental)

A local push-to-talk loop can keep audio, STT, and TTS on a Raspberry Pi while ZeroClaw owns channel orchestration, tool use, response policy, tracing, and handoff semantics. The recommended integration is the generic webhook channel:

```toml
[channels.webhook]
enabled = true
port = 42881
listen_path = "/voice"
send_url = "http://127.0.0.1:42882/tts"
secret = "change-me"

[channels.webhook.response_policy]
mode = "voice"
audio_safe = true
error_style = "natural"
allow_raw_errors = false
max_spoken_chars = 600

[pacing]
loop_detection_enabled = true
loop_detection_window_size = 20
loop_detection_max_repeats = 3
```

The local button loop sends a transcript plus optional voice metadata:

```json
{
  "sender": "rpi5-button",
  "content": "jaka jest pogoda",
  "thread_id": "local-voice",
  "voice_event": {
    "mode": "agent",
    "stt_backend": "sherpa",
    "stt_latency_ms": 812,
    "audio_id": "utt-42",
    "timing": { "record_ms": 1430, "queue_ms": 12 }
  }
}
```

Legacy/top-level metadata is also accepted (`mode`, `stt_backend`, `stt_ms`, `latency_ms`, `utterance_id`). ZeroClaw stores this as bounded schema-only voice-event attachment data for trace correlation, keeps it out of prompt-visible text, and writes it into `runtime_trace` as `voice_turn_timeline` events.

Outbound replies remain backward-compatible with text-only TTS callbacks by keeping `content` at the top level, and add a structured event envelope:

```json
{
  "content": "Już sprawdzam pogodę.",
  "recipient": "rpi5-button",
  "thread_id": "local-voice",
  "event": {
    "protocol": "zeroclaw.webhook.event",
    "type": "assistant_response",
    "version": 1,
    "content_format": "text/plain",
    "tts": {
      "speak": true,
      "format": "plain_text",
      "interrupt_thinking": true
    }
  }
}
```

For broad implementation requests from voice, the response policy asks the user whether to continue locally step by step or prepare a handoff to the main agent. For medium-risk or multi-step local actions, it asks for a short yes/no confirmation before acting.

Arm64 rollout path for Raspberry Pi:

1. Run the manual **Cross-Platform Build** workflow.
2. Download the `zeroclaw-aarch64-unknown-linux-gnu` artifact for Raspberry Pi OS / Debian arm64.
3. On the Pi, back up the current binary, for example `sudo cp /usr/local/bin/zeroclaw /usr/local/bin/zeroclaw.bak`.
4. Copy the new `zeroclaw` binary to `/usr/local/bin/zeroclaw` and make it executable.
5. Add the webhook `response_policy` config shown above and restart the voice channel service.
6. Test a short weather query, a broad implementation request, and a forced repeated-tool prompt before leaving it unattended.

See `dev/config.rpi-voice.toml` for a partial sample configuration.

## TTS (outbound speech synthesis)

```toml
[channels.tts]
enabled = true
engine = "piper"                   # local, free
voice = "en_US-lessac-medium"
output_device = "default"
```

Other engines:

```toml
[channels.tts]
engine = "openai"                  # TTS-1 or TTS-1-HD
voice = "alloy"
api_key = "..."

[channels.tts]
engine = "elevenlabs"
voice_id = "21m00Tcm4TlvDq8ikWAM"
api_key = "..."

[channels.tts]
engine = "google-cloud"
voice = "en-US-Neural2-J"
credentials_json = "~/.zeroclaw/gcp.json"

[channels.tts]
engine = "edge"                    # Microsoft Edge TTS — free, online
voice = "en-US-AndrewNeural"
```

TTS is output-only — it's not a conversation channel; it's a speaker for other channels' replies. Pair with `voice_wake` for a complete local voice assistant.

---

## Latency budget

Speech feels real-time below ~500 ms end-to-end. Practical budgets:

| Component | Typical latency |
|---|---|
| Wake detection (local) | <100 ms |
| STT (Whisper local) | 300–800 ms per utterance |
| LLM first-token | 100–2000 ms (model dependent) |
| TTS first-audio | 200–700 ms |
| Network (cellular / PSTN) | 100–300 ms RTT |

ClawdTalk shortcuts several of these by keeping the audio stream live; regular `voice_call` incurs STT + LLM + TTS sequentially.

## STT options

The runtime picks STT based on `stt_provider`:

- `whisper` — local whisper.cpp; CPU-only works but GPU helps
- `openai-whisper` — hosted Whisper API
- `deepgram` — real-time streaming STT (fastest)
- `google-cloud-stt`
- `azure-stt`

Set in the voice channel's config (`stt_provider = "deepgram"`).

## Hardware notes

For always-on voice on an SBC:

- USB mic: any UAC-compliant mic works. `arecord -l` to verify the OS sees it
- Speaker: either USB audio out or the SBC's onboard jack; set `output_device` to the ALSA name
- Microphones with built-in AEC (acoustic echo cancellation) dramatically improve wake reliability when the speaker is nearby

See [Hardware → Android](../hardware/android-setup.md) for Android-specific audio setup.
