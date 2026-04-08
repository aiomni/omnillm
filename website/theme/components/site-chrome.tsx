import { Link } from '@rspress/core/runtime';

const footerLinks = [
  { href: '/', label: 'Overview' },
  { href: '/usage', label: 'Usage' },
  { href: '/architecture', label: 'Architecture' },
  { href: '/implementation', label: 'Implementation' }
];

export function SiteNavTitle() {
  return (
    <Link className="omn-nav-brand" to="/">
      <span className="omn-nav-brand__mark" aria-hidden="true" />
      <span>OMNILLM</span>
    </Link>
  );
}

export function NavInstallButton() {
  return (
    <Link className="omn-nav-install" to="/usage#installation">
      Install
    </Link>
  );
}

export function SiteFooter() {
  return (
    <footer className="omn-site-footer">
      <div className="omn-site-footer__inner">
        <div className="omn-site-footer__lead">
          <Link className="omn-site-footer__brand" to="/">
            OMNILLM
          </Link>
          <p>
            Provider-neutral Rust runtime for routing, transcoding, replay-safe
            tracing, and budget-aware execution.
          </p>
        </div>

        <nav className="omn-site-footer__nav" aria-label="Footer">
          {footerLinks.map(link => (
            <Link key={link.href} to={link.href}>
              {link.label}
            </Link>
          ))}
          <a href="https://github.com/aiomni/omnillm" rel="noreferrer" target="_blank">
            GitHub
          </a>
        </nav>
      </div>
    </footer>
  );
}
