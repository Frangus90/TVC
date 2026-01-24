/**
 * Standardized error handling utility.
 * 
 * Provides consistent error handling patterns across the application.
 * All errors are logged and optionally displayed to users via toast notifications.
 * 
 * Usage:
 * ```ts
 * import { handleError, handleAsyncError } from './utils/errorHandler';
 * 
 * // In try/catch
 * try {
 *   await someOperation();
 * } catch (error) {
 *   handleError(error, 'Operation context', true); // true = show toast
 * }
 * 
 * // Wrapper function
 * const result = await handleAsyncError(
 *   () => someOperation(),
 *   'Operation context',
 *   true
 * );
 * ```
 */

import { logger } from "./logger";
import { showError } from "../stores/toast.svelte";

export interface AppError {
  message: string;
  code?: string;
  details?: unknown;
}

/**
 * Handle errors with consistent logging and user feedback
 */
export function handleError(
  error: unknown,
  context: string,
  showToast = true
): AppError {
  const appError: AppError = {
    message: error instanceof Error ? error.message : String(error),
    code: error instanceof Error ? error.name : "UnknownError",
    details: error,
  };

  // Always log errors
  logger.error(`[${context}] ${appError.message}`, appError.details);

  // Optionally show user-facing toast
  if (showToast) {
    showError(appError.message);
  }

  return appError;
}

/**
 * Handle async operation errors
 */
export async function handleAsyncError<T>(
  operation: () => Promise<T>,
  context: string,
  showToast = true
): Promise<T | null> {
  try {
    return await operation();
  } catch (error) {
    handleError(error, context, showToast);
    return null;
  }
}

/**
 * Wrap async functions with error handling
 */
export function withErrorHandling<T extends (...args: any[]) => Promise<any>>(
  fn: T,
  context: string,
  showToast = true
): T {
  return (async (...args: Parameters<T>) => {
    try {
      return await fn(...args);
    } catch (error) {
      handleError(error, context, showToast);
      throw error; // Re-throw to allow caller to handle if needed
    }
  }) as T;
}
