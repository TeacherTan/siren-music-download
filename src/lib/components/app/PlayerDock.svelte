<script lang="ts">
  import AudioPlayer from '$lib/components/AudioPlayer.svelte';

  interface Song {
    cid: string;
    name: string;
    artists: string[];
    coverUrl: string | null;
  }

  type RepeatMode = 'all' | 'one';
  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';

  interface Props {
    song: Song | null;
    isPlaying: boolean;
    isPaused: boolean;
    hasPrevious: boolean;
    hasNext: boolean;
    progress: number;
    duration: number;
    isLoading?: boolean;
    reducedMotion?: boolean;
    isShuffled?: boolean;
    repeatMode?: RepeatMode;
    lyricsActive?: boolean;
    playlistActive?: boolean;
    downloadState?: SongDownloadState;
    downloadDisabled?: boolean;
    onPrevious?: () => void;
    onTogglePlay?: () => void;
    onSeek?: (positionSecs: number) => void | Promise<void>;
    onNext?: () => void;
    onShuffleChange?: (next: boolean) => void | Promise<void>;
    onRepeatModeChange?: (next: RepeatMode) => void | Promise<void>;
    onToggleLyrics?: () => void;
    onTogglePlaylist?: () => void;
    onDownload?: () => void | Promise<void>;
  }

  let props: Props = $props();
</script>

<div class="pointer-events-none absolute inset-x-0 bottom-4 z-[140] px-6">
  <div
    class="pointer-events-auto mx-auto max-w-[760px] rounded-full border border-white/50 bg-[var(--surface-dock)] p-2 shadow-[0_18px_40px_rgba(15,23,42,0.18)] backdrop-blur-xl"
  >
    <AudioPlayer
      song={props.song}
      isPlaying={props.isPlaying}
      isPaused={props.isPaused}
      hasPrevious={props.hasPrevious}
      hasNext={props.hasNext}
      progress={props.progress}
      duration={props.duration}
      isLoading={props.isLoading}
      reducedMotion={props.reducedMotion}
      isShuffled={props.isShuffled}
      repeatMode={props.repeatMode}
      lyricsActive={props.lyricsActive}
      playlistActive={props.playlistActive}
      downloadState={props.downloadState}
      downloadDisabled={props.downloadDisabled}
      onPrevious={props.onPrevious}
      onTogglePlay={props.onTogglePlay}
      onSeek={props.onSeek}
      onNext={props.onNext}
      onShuffleChange={props.onShuffleChange}
      onRepeatModeChange={props.onRepeatModeChange}
      onToggleLyrics={props.onToggleLyrics}
      onTogglePlaylist={props.onTogglePlaylist}
      onDownload={props.onDownload}
    />
  </div>
</div>
