<script lang="ts">
  import { OverlayScrollbarsComponent } from 'overlayscrollbars-svelte';
  import type { PartialOptions } from 'overlayscrollbars';
  import AlbumSidebar from '$lib/components/app/AlbumSidebar.svelte';
  import type {
    Album,
    LibrarySearchScope,
    SearchLibraryResponse,
    SearchLibraryResultItem,
  } from '$lib/types';

  interface Props {
    isMacOS: boolean;
    albums: Album[];
    selectedAlbumCid: string | null;
    reducedMotion: boolean;
    loadingAlbums: boolean;
    errorMsg: string;
    searchQuery: string;
    searchScope: LibrarySearchScope;
    searchLoading: boolean;
    searchResponse: SearchLibraryResponse | null;
    overlayScrollbarOptions: PartialOptions;
    onSearchQueryChange: (query: string) => void;
    onSearchScopeChange: (scope: LibrarySearchScope) => void;
    onSelect: (album: Album) => void | Promise<void>;
    onSelectSearchResult: (
      item: SearchLibraryResultItem
    ) => void | Promise<void>;
  }

  let props: Props = $props();
</script>

<OverlayScrollbarsComponent
  element="aside"
  class="sidebar"
  data-overlayscrollbars-initialize
  options={props.overlayScrollbarOptions}
  defer
>
  {#if props.isMacOS}
    <div
      class="sidebar-drag-region"
      data-tauri-drag-region
      aria-hidden="true"
    ></div>
  {/if}
  <AlbumSidebar
    albums={props.albums}
    selectedAlbumCid={props.selectedAlbumCid}
    reducedMotion={props.reducedMotion}
    loadingAlbums={props.loadingAlbums}
    errorMsg={props.errorMsg}
    searchQuery={props.searchQuery}
    searchScope={props.searchScope}
    searchLoading={props.searchLoading}
    searchResponse={props.searchResponse}
    onSearchQueryChange={props.onSearchQueryChange}
    onSearchScopeChange={props.onSearchScopeChange}
    onSelect={props.onSelect}
    onSelectSearchResult={props.onSelectSearchResult}
  />
</OverlayScrollbarsComponent>
