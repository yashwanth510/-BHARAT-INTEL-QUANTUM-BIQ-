import { test, expect } from '@playwright/test';

test('map loads, layers toggle, and a click fetches weather at the clicked coordinate', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  const tile = page.waitForResponse(response => response.url().includes('.mvt') && response.ok());
  await page.goto('/');
  await tile;
  await expect(page.getByText('BIQ', { exact: true })).toBeVisible();
  await expect(page.locator('.maplibregl-canvas')).toBeVisible();
  await expect(page.locator('.conn-dot')).not.toHaveClass(/disconnected/, { timeout: 15000 });
  const flights = page.getByRole('button', {name: /flights/});
  await expect(flights).toHaveAttribute('aria-pressed', 'true');
  await flights.click();
  await expect(flights).toHaveAttribute('aria-pressed', 'false');
  await flights.click();
  const responsePromise = page.waitForResponse(response => response.url().includes('/api/weather?'));
  await page.mouse.click(560, 350);
  const response = await responsePromise;
  const url = new URL(response.url());
  expect(Number.isFinite(Number(url.searchParams.get('lat')))).toBe(true);
  const card = page.getByRole('region', {name: 'Weather at selected location'});
  await expect(card).toBeVisible();
  await expect(card).not.toContainText('Loading…', {timeout: 45000});
  await page.getByRole('button', {name: 'Close weather card'}).click();
  await expect(card).not.toBeVisible();
  await page.getByRole('button', {name: 'Satellite basemap'}).click();
  await expect(page.getByRole('button', {name: 'Street basemap'})).toBeVisible();
  expect(errors).toEqual([]);
  await page.screenshot({path:'test-results/map-desktop.png'});
});

test('weather errors stop the loading state', async ({ page }) => {
  await page.route('**/api/weather?*', route => route.fulfill({status:503, contentType:'application/json',body:JSON.stringify({error:'Weather test unavailable'})}));
  await page.goto('/');
  await expect(page.locator('.maplibregl-canvas')).toBeVisible();
  await page.getByRole('button', {name:/flights/}).click();
  await page.getByRole('button', {name:/anomalies/}).click();
  await page.mouse.click(550, 350);
  await expect(page.getByRole('alert').filter({hasText:'Weather test unavailable'})).toBeVisible();
});

test('mobile layout keeps layer controls accessible', async ({ page }) => {
  await page.setViewportSize({width:390,height:844});
  await page.goto('/');
  await expect(page.locator('.maplibregl-canvas')).toBeVisible();
  await expect(page.getByRole('button', {name:/anomalies/})).toBeInViewport();
  await page.screenshot({path:'test-results/map-mobile.png'});
});

test('fixture anomaly click sends its exact coordinates and renders imagery and citations', async ({ page }) => {
  const anomaly = {id:'fixture-anomaly', severity:'high', reasons:['not_on_allowlist'], entity:{kind:'vessel',id:'fixture-vessel',lat:22,lon:80}, zone_id:'test-zone',ts:new Date().toISOString()};
  await page.route('**/api/anomalies', route=>route.fulfill({json:[anomaly]}));
  for (const kind of ['flights','vessels','vehicles']) await page.route(`**/api/${kind}`, route=>route.fulfill({json:[]}));
  await page.routeWebSocket('**/ws/live', socket=>socket.send(JSON.stringify({type:'heartbeat',ts:new Date().toISOString()})));
  let satelliteBody: any, osintBody: any;
  await page.route('**/api/satellite/snapshot', route=>{
    satelliteBody=route.request().postDataJSON();
    return route.fulfill({json:{image_url:'data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="green"/></svg>',bbox:[79.95,21.95,80.05,22.05]}});
  });
  await page.route('**/api/osint/enrich', route=>{
    osintBody=route.request().postDataJSON();
    return route.fulfill({json:{results:[{title:'Fixture source',url:'https://example.com/source',snippet:'Test enrichment'}]}});
  });
  await page.goto('/');
  await expect(page.locator('.topbar')).toContainText('Anomalies1');
  await expect(page.locator('.maplibregl-canvas')).toBeVisible();
  // Initial map center is 80E, 22N, exactly the center of this 1280x720 viewport.
  await expect(async () => {
    await page.mouse.click(640,360);
    await expect(page.getByRole('complementary', {name:'Anomaly details'})).toBeVisible({timeout:2000});
  }).toPass({timeout:20000});
  await expect(page.getByRole('link', {name:'Fixture source'})).toHaveAttribute('href','https://example.com/source');
  await expect(page.getByAltText('Sentinel satellite image')).toBeVisible();
  expect(satelliteBody).toEqual({lat:22,lon:80});
  expect(osintBody).toEqual({lat:22,lon:80,entity:'fixture-vessel'});
  await expect(page.getByRole('region',{name:'Weather at selected location'})).not.toBeVisible();
  await page.getByRole('button',{name:'Close anomaly drawer'}).click();
  await expect(page.getByRole('complementary')).not.toBeVisible();
});

test('WebSocket close marks the connection offline', async ({page}) => {
  let connection: any;
  await page.routeWebSocket('**/ws/live', socket=>{connection=socket; socket.send(JSON.stringify({type:'heartbeat',ts:new Date().toISOString()}));});
  await page.goto('/');
  await expect(page.locator('.conn-dot')).not.toHaveClass(/disconnected/);
  connection.close();
  await expect(page.locator('.conn-dot')).toHaveClass(/disconnected/);
});
