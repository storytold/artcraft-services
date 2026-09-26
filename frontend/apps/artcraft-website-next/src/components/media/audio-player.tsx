"use client";

import { useEffect, useRef, useState, type CSSProperties } from "react";
import { Music2Icon, PauseIcon, PlayIcon, Volume2Icon, VolumeXIcon } from "lucide-react";
import styles from "./audio-player.module.css";
import AudioWaveform from "./audio-waveform";

export default function AudioPlayer({ src, title, onError }: {
  src: string; title: string; onError: () => void;
}) {
  const audio = useRef<HTMLAudioElement>(null);
  const [playing, setPlaying] = useState(false);
  const [duration, setDuration] = useState(0);
  const [position, setPosition] = useState(0);
  const [volume, setVolume] = useState(1);
  const [muted, setMuted] = useState(false);
  const [message, setMessage] = useState("");
  const audioGraph = useRef<{ context: AudioContext; source: MediaElementAudioSourceNode; analyser: AnalyserNode } | null>(null);
  const [analyser, setAnalyser] = useState<AnalyserNode | null>(null);
  useEffect(() => () => {
    const graph = audioGraph.current;
    graph?.source.disconnect();
    graph?.analyser.disconnect();
    if (graph) void graph.context.close();
  }, []);

  async function togglePlayback() {
    const element = audio.current;
    if (!element) return;
    setMessage("");
    if (!element.paused) { element.pause(); return; }
    // Initialize only on the user's play gesture so browser audio policies are
    // honored. Native playback still works when Web Audio isn't available.
    if (!audioGraph.current && typeof AudioContext !== "undefined") {
      try {
        const context = new AudioContext();
        const source = context.createMediaElementSource(element);
        const node = context.createAnalyser();
        node.fftSize = 2048;
        source.connect(node);
        node.connect(context.destination);
        audioGraph.current = { context, source, analyser: node };
        setAnalyser(node);
      } catch { /* The native player can continue without a waveform. */ }
    }
    if (audioGraph.current) await audioGraph.current.context.resume().catch(() => {});
    try { await element.play(); }
    catch { setMessage("Playback could not start. Press play to try again."); }
  }

  function seek(value: number) {
    if (!audio.current || !duration) return;
    audio.current.currentTime = value;
    setPosition(value);
  }

  function changeVolume(value: number) {
    if (!audio.current) return;
    audio.current.volume = value;
    audio.current.muted = false;
    setVolume(value);
    setMuted(false);
  }

  function toggleMute() {
    if (!audio.current) return;
    const next = !(muted || volume === 0);
    audio.current.muted = next;
    if (!next && volume === 0) { audio.current.volume = 1; setVolume(1); }
    setMuted(next);
  }

  const silent = muted || volume === 0;
  return (
    <div role="group" aria-label="Audio player" className="w-full max-w-xl px-6 py-12 sm:px-10">
      <audio ref={audio} src={src} crossOrigin="anonymous" preload="metadata" className="hidden" onError={onError}
        onLoadedMetadata={() => setDuration(Number.isFinite(audio.current?.duration) ? audio.current!.duration : 0)}
        onDurationChange={() => setDuration(Number.isFinite(audio.current?.duration) ? audio.current!.duration : 0)}
        onTimeUpdate={() => setPosition(audio.current?.currentTime ?? 0)}
        onPlay={() => setPlaying(true)} onPause={() => setPlaying(false)} onEnded={() => setPlaying(false)} />

      <div className="border border-line-strong bg-bg-raised">
        <div className="flex items-center justify-between border-b border-line px-5 py-3">
          <span className="hud-label text-muted">Audio / Original</span>
          <span className="hud-label flex items-center gap-2 text-accent-ink">
            <span aria-hidden className={`h-1.5 w-1.5 ${playing ? "bg-accent" : "border border-line-strong"}`} />
            {playing ? "Playing" : "Ready to play"}
          </span>
        </div>
        <div className="flex flex-col items-center px-6 py-10 sm:py-14">
          <div className="relative mb-8 flex h-32 w-32 items-center justify-center border border-line">
            <span aria-hidden className="tick -left-[5px] -top-[5px]" />
            <span aria-hidden className="tick -bottom-[5px] -right-[5px]" />
            <div className="flex h-24 w-24 items-center justify-center border border-line bg-bg-sunken">
              <Music2Icon aria-hidden className="h-10 w-10 text-accent-ink" strokeWidth={1} />
            </div>
          </div>
          <h2 className="w-full break-words text-center font-display text-xl leading-snug text-ink-strong">{title}</h2>
          <p className="hud-label mt-3 text-faint">Shared with ArtCraft</p>
        </div>

        <AudioWaveform analyser={analyser} playing={playing} />
        <div className="p-5 sm:p-6">
          <input type="range" aria-label="Seek audio" aria-valuetext={`${timeLabel(position)} of ${timeLabel(duration)}`}
            min={0} max={duration || 1} step={0.01} value={position} disabled={!duration}
            onChange={(event) => seek(Number(event.target.value))}
            style={{ "--progress": `${duration ? position / duration * 100 : 0}%` } as CSSProperties}
            className={`${styles.range} w-full`} />
          <div className="hud-label mt-2 flex justify-between text-muted">
            <output aria-label="Elapsed time">{timeLabel(position)}</output>
            <span aria-label="Track duration">{timeLabel(duration)}</span>
          </div>
          <div className="mt-5 flex items-center justify-between gap-4">
            <button type="button" aria-label={playing ? "Pause audio" : "Play audio"} onClick={togglePlayback}
              className="flex h-14 w-14 shrink-0 items-center justify-center bg-invert-bg text-invert-fg transition-opacity hover:opacity-80 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-accent-ink">
              {playing ? <PauseIcon aria-hidden className="h-5 w-5" fill="currentColor" /> : <PlayIcon aria-hidden className="h-5 w-5" fill="currentColor" />}
            </button>
            <div className="flex items-center gap-3">
              <button type="button" aria-label={silent ? "Unmute audio" : "Mute audio"} onClick={toggleMute}
                className="flex h-10 w-10 items-center justify-center text-muted hover:text-ink focus-visible:outline-2 focus-visible:outline-accent-ink">
                {silent ? <VolumeXIcon aria-hidden className="h-5 w-5" /> : <Volume2Icon aria-hidden className="h-5 w-5" />}
              </button>
              <input type="range" aria-label="Audio volume" aria-valuetext={`${Math.round((silent ? 0 : volume) * 100)}%`}
                min={0} max={1} step={0.01} value={silent ? 0 : volume} onChange={(event) => changeVolume(Number(event.target.value))}
                style={{ "--progress": `${(silent ? 0 : volume) * 100}%` } as CSSProperties} className={`${styles.range} w-20 sm:w-24`} />
            </div>
          </div>
          {message && <p role="status" className="mt-4 text-sm text-muted">{message}</p>}
        </div>
      </div>
    </div>
  );
}

function timeLabel(seconds: number): string {
  const value = Math.max(0, Math.floor(seconds));
  return `${Math.floor(value / 60)}:${String(value % 60).padStart(2, "0")}`;
}
