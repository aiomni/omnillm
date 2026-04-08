import './index.css';

import { preloadLink } from '@rspress/core/runtime';
import { Layout as BasicLayout } from '@rspress/core/theme-original';
import {
  DocIntro,
  DocsOutlineCta,
  DocsSidebarIntro
} from './components/doc-chrome';
import {
  NavInstallButton,
  SiteFooter,
  SiteNavTitle
} from './components/site-chrome';

const PRELOAD_ROUTES = ['/', '/usage', '/architecture', '/implementation'];
let didWarmRoutes = false;

function warmRoutes() {
  if (didWarmRoutes || typeof window === 'undefined') {
    return;
  }

  didWarmRoutes = true;

  for (const route of PRELOAD_ROUTES) {
    preloadLink(route);
  }
}

warmRoutes();

const Layout = () => {
  return (
    <BasicLayout
      navTitle={<SiteNavTitle />}
      beforeNavMenu={<NavInstallButton />}
      beforeDocContent={<DocIntro />}
      beforeSidebar={<DocsSidebarIntro />}
      afterOutline={<DocsOutlineCta />}
      bottom={<SiteFooter />}
    />
  );
};

export { Layout };
export * from '@rspress/core/theme-original';
