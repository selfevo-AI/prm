import type { Dialog, Entity, Workspace } from "./api";

export type Page =
  | "dashboard"
  | "self"
  | "relations"
  | "mental"
  | "digital"
  | "physical"
  | "decisions"
  | "ontology"
  | "audit";

/** What every workspace page receives from the shell. */
export interface PageContext {
  workspace: Workspace;
  entities: Entity[];
  byKind: (kind: string) => Entity[];
  /** Applies the search box and the global scene filter. */
  filter: (list: Entity[]) => Entity[];
  inScene: (entity: Entity) => boolean;
  scene: string;
  setScene: (scene: string) => void;
  search: string;
  onDialog: (dialog: Dialog) => void;
  /** Runs a mutation, refreshes the workspace and shows `message`; false on failure. */
  run: (work: () => Promise<unknown>, message: string) => Promise<boolean>;
  busy: boolean;
  go: (page: Page) => void;
  fail: (reason: unknown) => void;
  notify: (message: string) => void;
  refresh: () => Promise<void>;
}
export const find = (entities: Entity[], id: unknown) =>
  entities.find((entity) => entity.id === id);
