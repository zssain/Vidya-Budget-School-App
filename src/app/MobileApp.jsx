import { DesktopApp } from './DesktopApp.jsx';
import { mobileViews } from './views-mobile.js';
export function MobileApp() {
  return <DesktopApp views={mobileViews} />;
}
