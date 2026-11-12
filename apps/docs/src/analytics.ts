import ExecutionEnvironment from '@docusaurus/ExecutionEnvironment';

// Page views on Linkdean's own GoatCounter: no cookies, nothing stored in the browser.
// Skipped when the browser sends Do Not Track or Global Privacy Control.
const COUNT_URL = 'https://stats.linkdean.app/count';

type GoatCounter = {
  path?: () => string;
  count?: (opts: { path: string }) => void;
};

declare global {
  interface Window {
    goatcounter?: GoatCounter;
  }
}

const pagePath = (): string => location.host + location.pathname;

function optedOut(): boolean {
  const n = navigator as Navigator & { globalPrivacyControl?: boolean };
  return n.doNotTrack === '1' || n.globalPrivacyControl === true;
}

if (ExecutionEnvironment.canUseDOM && !optedOut()) {
  window.goatcounter = { path: pagePath };
  const s = document.createElement('script');
  s.async = true;
  s.dataset.goatcounter = COUNT_URL;
  s.src = 'https://stats.linkdean.app/count.js';
  document.head.appendChild(s);
}

// count.js counts the first page; later pages are client-side navigations.
export function onRouteDidUpdate({
  location,
  previousLocation,
}: {
  location: Location;
  previousLocation: Location | null;
}): void {
  if (!previousLocation || location.pathname === previousLocation.pathname) return;
  window.goatcounter?.count?.({ path: pagePath() });
}
