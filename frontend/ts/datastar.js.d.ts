export interface ActionPlugin {
  type: 'action';
  name: string;
  fn: (...args: unknown[]) => unknown;
}

export function apply(target: Element | DocumentFragment | null): void;
export function load(...plugins: ActionPlugin[]): void;
