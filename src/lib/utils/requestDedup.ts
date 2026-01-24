/**
 * Request deduplication utility
 * Prevents duplicate API calls when multiple components request the same data
 */

interface PendingRequest<T> {
  promise: Promise<T>;
  timestamp: number;
}

class RequestDeduplicator {
  private pendingRequests = new Map<string, PendingRequest<any>>();
  private readonly requestTimeout = 60000; // 60 seconds

  /**
   * Deduplicate a request by key
   * If a request with the same key is already pending, returns the existing promise
   */
  async deduplicate<T>(
    key: string,
    requestFn: () => Promise<T>
  ): Promise<T> {
    // Check if there's a pending request
    const pending = this.pendingRequests.get(key);
    if (pending) {
      // Check if request is still valid (not too old)
      const age = Date.now() - pending.timestamp;
      if (age < this.requestTimeout) {
        return pending.promise;
      } else {
        // Request is too old, remove it
        this.pendingRequests.delete(key);
      }
    }

    // Create new request
    const promise = requestFn()
      .then((result) => {
        // Remove from pending when complete
        this.pendingRequests.delete(key);
        return result;
      })
      .catch((error) => {
        // Remove from pending on error
        this.pendingRequests.delete(key);
        throw error;
      });

    this.pendingRequests.set(key, {
      promise,
      timestamp: Date.now(),
    });

    return promise;
  }

  /**
   * Clear all pending requests
   */
  clear(): void {
    this.pendingRequests.clear();
  }

  /**
   * Clear a specific pending request
   */
  clearKey(key: string): void {
    this.pendingRequests.delete(key);
  }
}

// Export singleton instance
export const requestDeduplicator = new RequestDeduplicator();

/**
 * Create a deduplicated request function
 */
export function createDeduplicatedRequest<T>(
  keyGenerator: (...args: any[]) => string,
  requestFn: (...args: any[]) => Promise<T>
) {
  return async (...args: Parameters<typeof requestFn>): Promise<T> => {
    const key = keyGenerator(...args);
    return requestDeduplicator.deduplicate(key, () => requestFn(...args));
  };
}
