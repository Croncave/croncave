import { expect, test } from '@playwright/test';
import { freshEmail, signInLinkFor } from './helpers';

async function signIn(page: import('@playwright/test').Page) {
  const email = freshEmail();
  await page.goto('/sign-in');
  await page.getByLabel('Email').fill(email);
  await page.getByRole('button', { name: 'Send sign-in link' }).click();
  await expect(page.getByText('Check')).toContainText(email);
  await page.goto(await signInLinkFor(email));
  await expect(page).toHaveURL('/');
}

test('a workspace connects by itself, and runs what it is asked', async ({ page }) => {
  test.slow(); // a container has to start and dial home

  await signIn(page);

  await test.step('make one and open it', async () => {
    await page.goto('/workspaces/new');
    await page.getByLabel('Name').fill('Stock Watcher');
    await page.getByRole('button', { name: 'Create workspace' }).click();
    await page
      .getByRole('listitem')
      .filter({ hasText: 'Stock Watcher' })
      .getByRole('link')
      .click();
  });

  await test.step('asleep, it is not connected', async () => {
    await expect(page.getByRole('definition').filter({ hasText: 'no' })).toBeVisible();
  });

  await test.step('waking it brings the agent up, all by itself', async () => {
    await page.getByRole('button', { name: 'Wake it' }).click();

    // Nothing reaches into the workspace: it dials out when it starts, so
    // this is the app waiting to be called rather than connecting.
    await expect(async () => {
      await page.reload();
      await expect(page.getByText('Run a command')).toBeVisible({ timeout: 2000 });
    }).toPass({ timeout: 90_000 });
  });

  await test.step('it runs a command and the output comes back', async () => {
    await page.getByRole('textbox').fill('echo hello from inside');
    await page.getByRole('button', { name: 'Run' }).click();

    await expect(page.locator('pre')).toContainText('hello from inside');
    await expect(page.getByText('done')).toBeVisible();
  });

  await test.step('a command that fails says so', async () => {
    await page.getByRole('textbox').fill('false');
    await page.getByRole('button', { name: 'Run' }).click();

    await expect(page.getByText('failed')).toBeVisible();
  });

  await test.step('putting it to sleep takes the connection with it', async () => {
    await page.getByRole('button', { name: 'Put it to sleep' }).click();
    await expect(page.getByRole('heading', { level: 1 }).locator('..')).toContainText('asleep');
    await expect(page.getByText('Run a command')).toBeHidden();
  });
});
