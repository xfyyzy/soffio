interface ComponentDefinition {
  tag: string;
  role?: string;
  ariaLevel?: number;
  labelAttr?: string;
  defaultLabel?: string;
}

const componentDefinitions: ComponentDefinition[] = [
  { tag: 'page-shell', role: 'document' },
  { tag: 'header-bar', role: 'presentation' },
  { tag: 'layout-frame', role: 'group' },
  { tag: 'content-panel', role: 'region', labelAttr: 'aria-label', defaultLabel: 'Primary content' },
  { tag: 'post-grid', role: 'list' },
  { tag: 'post-card', role: 'listitem' },
  { tag: 'card-heading', role: 'presentation' },
  { tag: 'card-excerpt', role: 'presentation' },
  { tag: 'card-footer', role: 'contentinfo' },
  { tag: 'tag-badges', role: 'list' },
  { tag: 'tag-chip-group', role: 'listitem' },
  { tag: 'chip-count', role: 'status' },
  { tag: 'empty-state', role: 'status' },
  { tag: 'empty-title', role: 'heading', ariaLevel: 2 },
  { tag: 'empty-copy', role: 'note' },
  { tag: 'load-more', role: 'presentation' },
  { tag: 'filters-column', role: 'complementary', labelAttr: 'aria-label', defaultLabel: 'Filters' },
  { tag: 'meta-column', role: 'complementary', labelAttr: 'aria-label', defaultLabel: 'Article metadata' },
  { tag: 'meta-card', role: 'group' },
  { tag: 'post-summary', role: 'note' },
  { tag: 'post-section', role: 'region', labelAttr: 'aria-label', defaultLabel: 'Article section' },
];

const defineSemanticElement = ({ tag, role, ariaLevel, labelAttr, defaultLabel }: ComponentDefinition): void => {
  if (customElements.get(tag)) {
    return;
  }

  class SemanticElement extends HTMLElement {
    connectedCallback(): void {
      if (role && !this.hasAttribute('role')) {
        this.setAttribute('role', role);
      }

      if (ariaLevel && !this.hasAttribute('aria-level')) {
        this.setAttribute('aria-level', String(ariaLevel));
      }

      if (labelAttr && defaultLabel && !this.hasAttribute(labelAttr)) {
        this.setAttribute(labelAttr, defaultLabel);
      }
    }
  }

  customElements.define(tag, SemanticElement);
};

for (const definition of componentDefinitions) {
  defineSemanticElement(definition);
}
