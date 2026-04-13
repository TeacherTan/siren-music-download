export interface Album {
  cid: string;
  name: string;
  coverUrl: string;
  artists: string[];
}

export interface SongEntry {
  cid: string;
  name: string;
  artists: string[];
}

export interface SongDetail {
  cid: string;
  name: string;
  albumCid: string;
  sourceUrl: string;
  lyricUrl: string | null;
  mvUrl: string | null;
  mvCoverUrl: string | null;
  artists: string[];
}

export interface AlbumDetail {
  cid: string;
  name: string;
  intro: string | null;
  belong: string;
  coverUrl: string;
  coverDeUrl: string | null;
  artists: string[] | null;
  songs: SongEntry[];
}

export type OutputFormat = 'flac' | 'wav' | 'mp3';

export interface PlayerState {
  songCid: string | null;
  songName: string | null;
  artists: string[];
  coverUrl: string | null;
  isPlaying: boolean;
  isLoading: boolean;
  progress: number;
  duration: number;
}
