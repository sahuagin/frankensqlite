import type { SerializedFrankenError } from "@frankensqlite/worker";

export class FrankenSQLiteError extends Error {
  readonly code: string;
  readonly sqliteCode?: number;
  readonly extendedCode?: number;
  readonly transient?: boolean;
  readonly userRecoverable?: boolean;
  readonly suggestion?: string;

  constructor(error: SerializedFrankenError) {
    super(error.message);
    this.name = "FrankenSQLiteError";
    this.code = error.code;
    this.sqliteCode = error.sqliteCode;
    this.extendedCode = error.extendedCode;
    this.transient = error.transient;
    this.userRecoverable = error.userRecoverable;
    this.suggestion = error.suggestion;
    if (error.stack) {
      this.stack = error.stack;
    }
  }
}
