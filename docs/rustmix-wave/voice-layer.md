# Rustmix-Wave Voice Layer Direction

The Waveshare ESP32-S3 e-Paper 3.97 board is a better target for voice features
than the Xteink X4 because the product direction includes microphone/speaker and
assistant-style interaction.

## Voice interaction model

- Rotate: select menu item.
- Press: open menu item.
- Hold: push-to-talk voice capture.
- Release/press: stop capture and process.
- Display assistant response text on e-paper.
- Optionally play spoken response later.

## Recommended phases

1. Voice UI states only:
   - Idle
   - Listening
   - Processing
   - Reply ready
   - Offline

2. Audio codec bring-up:
   - speaker test
   - microphone capture
   - WAV record/play from SD

3. Network assistant:
   - send captured audio or text request over Wi-Fi
   - display response

4. Device actions:
   - open reader
   - show weather
   - set timer
   - summarize today
