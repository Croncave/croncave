import { expect, test } from '@playwright/test';
import { freshEmail, signInLinkFor } from './helpers';

test('someone signs in, makes a workspace, and signs out', async ({ page }) => {
  const email = freshEmail();

  await test.step('signed out, the app asks you to sign in', async () => {
    await page.goto('/');
    await expect(page).toHaveURL(/\/sign-in$/);
  });

  await test.step('asking for a link says so without saying whether you exist', async () => {
    await page.getByLabel('Email').fill(email);
    await page.getByRole('button', { name: 'Send sign-in link' }).click();
    await expect(page.getByText('Check')).toContainText(email);
  });

  const link = await signInLinkFor(email);

  await test.step('following the link signs you in', async () => {
    await page.goto(link);
    await expect(page).toHaveURL('/');
    await expect(page.getByRole('heading', { level: 1 })).toContainText('Welcome');
  });

  await test.step('there are no workspaces yet', async () => {
    await expect(page.getByText('Nothing here yet.')).toBeVisible();
  });

  await test.step('making one puts it on the page', async () => {
    await page.getByRole('link', { name: 'New workspace' }).first().click();
    await expect(page).toHaveURL(/\/workspaces\/new$/);

    await page.getByLabel('Name').fill('Stock Watcher');
    await page.getByRole('button', { name: 'Create workspace' }).click();

    await expect(page).toHaveURL('/');
    await expect(page.getByRole('listitem').filter({ hasText: 'Stock Watcher' })).toBeVisible();
    await expect(page.getByRole('navigation', { name: 'Main' })).toContainText('Stock Watcher');
  });

  await test.step('signing out ends it', async () => {
    await page.getByRole('button', { name: 'Sign out' }).click();
    await expect(page).toHaveURL(/\/sign-in$/);

    await page.goto('/');
    await expect(page).toHaveURL(/\/sign-in$/);
  });
});

test('a sign-in link works exactly once', async ({ page }) => {
  const email = freshEmail();

  await page.goto('/sign-in');
  await page.getByLabel('Email').fill(email);
  await page.getByRole('button', { name: 'Send sign-in link' }).click();
  await expect(page.getByText('Check')).toContainText(email);

  const link = await signInLinkFor(email);

  await page.goto(link);
  await expect(page).toHaveURL('/');

  // Sign out so the second attempt is judged on the link, not on the session
  // the browser is still carrying.
  await page.getByRole('button', { name: 'Sign out' }).click();
  await expect(page).toHaveURL(/\/sign-in$/);

  await page.goto(link);
  await expect(page).toHaveURL(/link=expired/);
  await expect(page.getByRole('alert')).toContainText('already been used');
});

test('a made-up link signs nobody in', async ({ page }) => {
  await page.goto('/auth/callback?token=not-a-real-token');

  await expect(page).toHaveURL(/link=expired/);
  await expect(page.getByRole('alert')).toBeVisible();
});

test('a workspace needs a name', async ({ page }) => {
  const email = freshEmail();

  await page.goto('/sign-in');
  await page.getByLabel('Email').fill(email);
  await page.getByRole('button', { name: 'Send sign-in link' }).click();
  await expect(page.getByText('Check')).toContainText(email);
  await page.goto(await signInLinkFor(email));

  await page.goto('/workspaces/new');
  await page.getByRole('button', { name: 'Create workspace' }).click();

  // The browser's own required-field check stops it before the server sees it.
  await expect(page).toHaveURL(/\/workspaces\/new$/);
});
