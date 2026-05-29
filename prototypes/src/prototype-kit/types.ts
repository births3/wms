export type PrototypeEnd = "pc" | "pda" | "pad" | "h5";
export type PrototypePriority = "P0" | "P1" | "P2" | "P3" | "P4";

export interface MatrixPrototypeSpec {
  storyId: string;
  title: string;
  end: PrototypeEnd;
  reason: string;
  priority: PrototypePriority;
  group: string;
  moduleCode: string;
  slug: string;
}
