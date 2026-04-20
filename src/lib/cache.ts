/**
 * TTL-based cache for API responses.
 * Entries expire after CACHE_TTL_MS (6 hours).
 */

const CACHE_TTL_MS = 6 * 60 * 60 * 1000; // 6 hours in milliseconds

interface CacheEntry<T> {
  data: T;
  timestamp: number;
}

// In-memory cache store
const cache = new Map<string, CacheEntry<unknown>>();

/**
 * Check if a cache entry is still valid (within TTL).
 */
function isEntryValid(entry: CacheEntry<unknown>): boolean {
  return Date.now() - entry.timestamp < CACHE_TTL_MS;
}

/**
 * Get cached data if available and not expired.
 */
export function getCached<T>(key: string): T | null {
  const entry = cache.get(key);
  if (entry && isEntryValid(entry)) {
    return entry.data as T;
  }
  // Remove expired entry
  if (entry) {
    cache.delete(key);
  }
  return null;
}

/**
 * Store data in cache with current timestamp.
 */
export function setCached<T>(key: string, data: T): void {
  cache.set(key, {
    data,
    timestamp: Date.now(),
  });
}

/**
 * Clear all cached entries.
 */
export function clearCache(): void {
  cache.clear();
}

/**
 * Remove all cached entries whose key starts with the given prefix.
 */
export function clearCachedByPrefix(prefix: string): void {
  for (const key of cache.keys()) {
    if (key.startsWith(prefix)) {
      cache.delete(key);
    }
  }
}

/**
 * Get cache statistics (for debugging/UI).
 */
export function getCacheStats(): { size: number; oldestTimestamp: number | null } {
  let oldest: number | null = null;
  for (const entry of cache.values()) {
    if (oldest === null || entry.timestamp < oldest) {
      oldest = entry.timestamp;
    }
  }
  return {
    size: cache.size,
    oldestTimestamp: oldest,
  };
}