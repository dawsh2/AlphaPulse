/**
 * Hashing utilities for manifest caching
 */

/**
 * Generate SHA256 hash
 */
export async function sha256(text: string): Promise<string> {
  const msgBuffer = new TextEncoder().encode(text);
  const hashBuffer = await crypto.subtle.digest('SHA-256', msgBuffer);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hashHex = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
  return hashHex;
}

/**
 * Generate manifest hash for caching
 */
export async function generateManifestHash(manifest: any): Promise<string> {
  // Sort keys for consistent hashing
  const sortedManifest = sortObjectKeys(manifest);
  const manifestString = JSON.stringify(sortedManifest);
  return sha256(manifestString);
}

/**
 * Sort object keys recursively for consistent hashing
 */
function sortObjectKeys(obj: any): any {
  if (obj === null || typeof obj !== 'object') {
    return obj;
  }
  
  if (Array.isArray(obj)) {
    return obj.map(sortObjectKeys);
  }
  
  const sorted: any = {};
  Object.keys(obj).sort().forEach(key => {
    sorted[key] = sortObjectKeys(obj[key]);
  });
  
  return sorted;
}

/**
 * Generate a simple hash for non-crypto purposes
 */
export function simpleHash(str: string): number {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash = hash & hash; // Convert to 32bit integer
  }
  return Math.abs(hash);
}

/**
 * Generate a unique ID
 */
export function generateId(prefix?: string): string {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substr(2, 9);
  return prefix ? `${prefix}_${timestamp}${random}` : `${timestamp}${random}`;
}

/**
 * Generate cache key from multiple parts
 */
export function generateCacheKey(...parts: any[]): string {
  return parts
    .map(part => {
      if (typeof part === 'object') {
        return JSON.stringify(sortObjectKeys(part));
      }
      return String(part);
    })
    .join(':');
}