import { expect, test } from '@playwright/test';
import { freshEmail, signInLinkFor } from './helpers';

/** Sign in and land on home. */
async function signIn(page: import('@playwright/test').Page) {
  const email = freshEmail();
  await page.goto('/sign-in');
  await page.getByLabel('Email').fill(email);
  await page.getByRole('button', { name: 'Send sign-in link' }).click();
  await expect(page.getByText('Check')).toContainText(email);
  await page.goto(await signInLinkFor(email));
  await expect(page).toHaveURL('/');
}

/** Make a workspace and open it. */
async function makeWorkspace(page: import('@playwright/test').Page, name: string) {
  await page.goto('/workspaces/new');
  await page.getByLabel('Name').fill(name);
  await page.getByRole('button', { name: 'Create workspace' }).click();
  await expect(page).toHaveURL('/');
  await page.getByRole('listitem').filter({ hasText: name }).getByRole('link').click();
  await expect(page.getByRole('heading', { level: 1 })).toContainText(name);
}

test('a workspace wakes and goes back to sleep', async ({ page }) => {
  await signIn(page);
  await makeWorkspace(page, 'Stock Watcher');

  await test.step('it starts asleep, with no computer made for it', async () => {
    await expect(page.getByRole('heading', { level: 1 }).locator('..')).toContainText('asleep');
  });

  await test.step('waking it makes one', async () => {
    await page.getByRole('button', { name: 'Wake it' }).click();
    await expect(page.getByRole('heading', { level: 1 }).locator('..')).toContainText('awake');
  });

  await test.step('the sidebar agrees', async () => {
    await expect(page.getByRole('navigation', { name: 'Main' })).toContainText('Stock Watcher');
  });

  await test.step('putting it to sleep stops it', async () => {
    await page.getByRole('button', { name: 'Put it to sleep' }).click();
    await expect(page.getByRole('heading', { level: 1 }).locator('..')).toContainText('asleep');
  });

  await test.step('and the state survives a reload, because it is asked for', async () => {
    await page.reload();
    await expect(page.getByRole('heading', { level: 1 }).locator('..')).toContainText('asleep');
  });
});

test('one team cannot open another team\'s workspace', async ({ page }) => {
  await signIn(page);
  await makeWorkspace(page, 'Private Plans');
  const url = page.url();

  // A different person, holding the exact address.
  await page.getByRole('button', { name: 'Sign out' }).click();
  await expect(page).toHaveURL(/\/sign-in$/);
  await signIn(page);

  await page.goto(url);
  await expect(page.getByText('No such workspace')).toBeVisible();
});
