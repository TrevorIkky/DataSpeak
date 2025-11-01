import { toast } from "sonner";

export class ErrorHandler {
  /**
   * Handle an error and display it to the user via toast
   */
  static handle(error: unknown, context?: string): void {
    const message = this.extractMessage(error);
    const fullMessage = context ? `${context}: ${message}` : message;

    toast.error(fullMessage, {
      duration: 5000,
    });

    // Log to console for debugging
    console.error(`[ErrorHandler] ${context || "Error"}:`, error);
  }

  /**
   * Display a success message
   */
  static success(message: string, description?: string): void {
    toast.success(message, {
      description,
      duration: 3000,
    });
  }

  /**
   * Display an info message
   */
  static info(message: string, description?: string): void {
    toast.info(message, {
      description,
      duration: 3000,
    });
  }

  /**
   * Display a warning message
   */
  static warning(message: string, description?: string): void {
    toast.warning(message, {
      description,
      duration: 4000,
    });
  }

  /**
   * Display a loading toast and return its ID for dismissal
   */
  static loading(message: string): string | number {
    return toast.loading(message);
  }

  /**
   * Dismiss a specific toast by ID
   */
  static dismiss(toastId: string | number): void {
    toast.dismiss(toastId);
  }

  /**
   * Extract a readable error message from various error types
   */
  private static extractMessage(error: unknown): string {
    if (typeof error === "string") {
      return error;
    }

    if (error instanceof Error) {
      return error.message;
    }

    if (error && typeof error === "object") {
      if ("message" in error && typeof error.message === "string") {
        return error.message;
      }

      if ("error" in error && typeof error.error === "string") {
        return error.error;
      }

      // Try to stringify the error object
      try {
        return JSON.stringify(error);
      } catch {
        return "An unknown error occurred";
      }
    }

    return "An unknown error occurred";
  }

  /**
   * Handle async operations with automatic error handling
   */
  static async handleAsync<T>(
    operation: () => Promise<T>,
    context?: string,
    successMessage?: string
  ): Promise<T | null> {
    try {
      const result = await operation();

      if (successMessage) {
        this.success(successMessage);
      }

      return result;
    } catch (error) {
      this.handle(error, context);
      return null;
    }
  }

  /**
   * Wrap an async operation with a loading toast
   */
  static async withLoading<T>(
    operation: () => Promise<T>,
    loadingMessage: string,
    successMessage?: string,
    errorContext?: string
  ): Promise<T | null> {
    const toastId = this.loading(loadingMessage);

    try {
      const result = await operation();
      this.dismiss(toastId);

      if (successMessage) {
        this.success(successMessage);
      }

      return result;
    } catch (error) {
      this.dismiss(toastId);
      this.handle(error, errorContext);
      return null;
    }
  }
}
