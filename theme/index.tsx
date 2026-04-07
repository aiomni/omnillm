import './index.css';

import { Layout as BasicLayout } from '@rspress/core/theme-original';

const Layout = () => (
  <BasicLayout
    beforeNavTitle={
      <span className="omn-nav-chip" aria-hidden="true">
        Rust runtime fabric
      </span>
    }
  />
);

export { Layout };
export * from '@rspress/core/theme-original';
