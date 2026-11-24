import { expect, test } from '@playwright/test';

test('initial focus leaves navigation available for keyboard use', async ({ page }) => {
  await page.goto('/main.html');

  await expect(page.locator('#root')).toBeFocused();
  const agent = page.getByRole('button', { name: 'Agent', exact: true });
  await expect(agent).not.toBeFocused();

  await page.keyboard.press('Tab');
  await expect(agent).toBeFocused();
  await expect(agent).toHaveCSS('outline-style', 'solid');
});
