# Manual setup guide

Status: last updated 2026-11-15. This guide moves to `linkdean-infra` with this folder in Phase 1.

Every step here is yours to do by hand: accounts, consoles, tokens and copying secrets. Claude does everything else in [the plan](README.md) and tells you when each step is due.

The steps are numbered in the order the phases need them. Each one names the page, the field and where its value comes from, and says what success looks like.

## Ground rules

- **Never paste a secret into a chat or a file.** Each command below copies a secret straight from where it is shown (the clipboard, a downloaded file, Infisical or a command's output) into GitHub or Secret Manager. Nothing is printed to the screen.
- **Values that are not secret** can be sent to Claude when a step asks: the Google Cloud project ID, the Cloudflare account ID and the SES DKIM records.
- **Commands are for zsh** on your Mac, and also work in bash.
- **More than one GitHub account?** If `gh` is signed in to more than one, put `GH_TOKEN=$(gh auth token --user arsamsarabi)` in front of each `gh` command.

| Step | When | What |
| --- | --- | --- |
| 1 | Before Phase 1 | Cloudflare token for Pages deploys |
| 2 | Phase 1b | Pages custom domains |
| 3–6 | Before Phase 3 | Tools, Google Cloud project, billing, budget, pricing check |
| 7–9 | Before Phase 3 | AWS account, MFA, budget and console user |
| 10 | Before Phase 3 | Cloudflare token for OpenTofu |
| 11–14 | Phase 3 | State bucket, running OpenTofu, SES identity, SES production access |
| 15–17 | Phase 4 | Database user, tunnel token, Meilisearch key |
| 18–20 | Phase 6 | Secrets from Infisical, SES SMTP login, Access |
| 21 | Phase 8 | Cutover day |
| 22 | Phase 9 | Retire the homelab beta |

## Before Phase 1

### 1. Cloudflare token for Pages deploys

`linkdean-infra` uses this token to upload the two sites. It can only edit Pages projects.

1. In the Cloudflare dashboard, choose your account, then **Manage account** → **Account API tokens** → **Create Token**.
2. Next to **Create Custom Token**, select **Get started**.
3. Fill in the form:
   - **Token name:** `linkdean-sites-pages`.
   - **Permissions:** one row, Account · **Cloudflare Pages** · **Edit**.
   - **TTL:** an end date one year from today. Put a reminder in your calendar to renew it.
4. Select **Continue to summary**, then **Create Token**. Cloudflare shows the token once. Copy it.
5. Wait until Claude says `linkdean-infra` exists. Then, with the token still on the clipboard, run:

   ```sh
   pbpaste | gh secret set CLOUDFLARE_API_TOKEN -R arsamsarabi/linkdean-infra && pbcopy < /dev/null
   ```

   `gh` reads the value from the pipe, so it does not wait for input. The last part clears the clipboard.
6. Send Claude your Cloudflare account ID, which is not secret. It is the 32-character code in the dashboard address, right after `dash.cloudflare.com/`.

**Success:** `gh secret list -R arsamsarabi/linkdean-infra` lists `CLOUDFLARE_API_TOKEN`.

## Phase 1b

### 2. Pages custom domains

Do this when Claude says the `*.pages.dev` previews match the live sites.

1. In the Cloudflare dashboard, go to **Workers & Pages** → **linkdean-site** → **Custom domains** → **Set up a domain**.
2. Enter `linkdean.app` and select **Continue**. Then confirm the DNS record Cloudflare proposes.
3. If Cloudflare says a record already exists for that name:
   1. Open **DNS** → **Records** for `linkdean.app` in another tab.
   2. Find the record with that name, and send Claude its **Type** and **Content**. That is the homelab tunnel, and the rollback.
   3. Delete the record, then go back and retry.

   The site is unreachable for the few seconds in between.
4. Repeat steps 1 to 3 for `www.linkdean.app` on **linkdean-site**, then for `docs.linkdean.app` on **linkdean-docs**.

**Success:** each domain shows as active on its project's **Custom domains** tab. Each site also loads in a private window, and `curl -s https://docs.linkdean.app/release.txt` prints the release Claude names.

Activation can take a few minutes.

## Before Phase 3: Google Cloud

### 3. Install the tools

1. Check what is already installed:

   ```sh
   command -v gcloud tofu
   ```

   Skip any tool that prints a path.
2. Install the rest:

   ```sh
   brew update
   brew install --cask gcloud-cli
   brew install opentofu
   ```

3. Sign in to Google twice. Each command opens a browser, where you pick your Google account:

   ```sh
   gcloud auth login
   gcloud auth application-default login
   ```

   The second login is the one OpenTofu uses.

**Success:** `gcloud --version` and `tofu version` each print a version.

### 4. Google Cloud project and billing

This is a new project, separate from the one you use for the extension stores.

1. Create the project:

   ```sh
   gcloud projects create linkdean-prod --name="Linkdean production"
   ```

   Project IDs are unique across all of Google Cloud. If this one is taken, pick another, such as `linkdean-prod-uk`, and use it wherever this guide says `linkdean-prod`. Tell Claude which ID you chose.
2. Find your billing account:

   ```sh
   gcloud billing accounts list
   ```

   - **The list shows an open account:** copy its `ACCOUNT_ID`, which looks like `012345-6789AB-CDEF01`.
   - **The list is empty:** in the console, go to **Navigation menu** → **Billing** → **Manage billing accounts** → **Create account**. If Google offers the free trial ($300 for 90 days), accept it.

     Then select **Activate full account** straight away. Otherwise the project stops when the trial ends, and the unused credit carries over anyway.
3. Link the billing account and switch on the APIs OpenTofu needs to start. OpenTofu switches on the rest itself.

   ```sh
   gcloud billing projects link linkdean-prod --billing-account=ACCOUNT_ID
   gcloud services enable cloudresourcemanager.googleapis.com serviceusage.googleapis.com \
     storage.googleapis.com --project=linkdean-prod
   gcloud auth application-default set-quota-project linkdean-prod
   ```

   Every command in this guide passes `--project`, so you do not need to change your default project.

**Success:** `gcloud billing projects describe linkdean-prod` shows `billingEnabled: true`.

### 5. Google Cloud budget

A budget only sends alerts. It does not cap spending. It stays a manual setting, outside OpenTofu.

1. In the console, choose the `linkdean-prod` project in the top bar. Then go to **Navigation menu** → **Billing** → **Cost management** → **Budgets & alerts** → **Create budget**.
2. If asked what kind of budget, choose **Alerts only**.
3. Fill in the form:
   - **Name:** `linkdean-prod monthly`. Select **Next**.
   - **Time range:** Monthly. **Projects:** `linkdean-prod`. Select **Next**.
   - **Budget type:** Specified amount. **Target amount:** 80 if the billing account is in dollars, or 65 if it is in pounds. Select **Next**.
   - **Thresholds:** keep 50%, 90% and 100% on **Actual**. Select **Add threshold** and add 100% on **Forecasted**.
   - Keep **Email alerts to billing admins and users** ticked.
4. Select **Finish**.

**Success:** the budget appears on **Budgets & alerts**. The first alert email can take a few hours to start working.

### 6. Pricing check

1. Open the [Google Cloud pricing calculator](https://cloud.google.com/products/calculator).
2. Add these items, all in `europe-west2`:

   | Product | Settings |
   | --- | --- |
   | Compute Engine | `e2-medium`, 730 hours, a 10 GB balanced boot disk, a 20 GB balanced data disk and 20 GB of snapshots |
   | Cloud SQL | PostgreSQL, Enterprise edition, `db-f1-micro`, 10 GB SSD, 10 GB of backups |
   | Cloud NAT | One gateway, 5 GB of data a month |

3. Send Claude the monthly total, or the calculator's share link.

**Success:** Claude confirms the total against the plan's $55–80, or updates the plan.

## Before Phase 3: AWS (for email)

### 7. AWS account

1. Go to [aws.amazon.com](https://aws.amazon.com) and select **Create an AWS Account**.
2. Fill in the first page:
   - **Root user email address:** an address you control and will keep. The account is recovered through it.
   - **AWS account name:** `linkdean`.
3. Enter the code AWS emails you, then set a root password and store it in your password manager.
4. When asked to choose an account plan, choose the **Paid plan**.
   - A Free plan account closes after six months, or when its credits run out.
   - SES costs a few cents a month here.
   - The $100 sign-up credit applies on either plan.
5. Contact information: choose **Personal**, unless you want the account in a business name.
6. Add a payment card, then verify your phone number with the code AWS sends.
7. For the support plan, choose **Basic support - Free**, then complete the sign-up.

**Success:** you can sign in to the console as the root user.

### 8. Root MFA, region and budget

1. In the console, open the region menu at the top right and choose **Europe (London) eu-west-2**. Steps 13, 14 and 19 must happen in this region.
2. Add MFA to the root user:
   1. Open your account name at the top right, then **Security credentials**.
   2. Under **Multi-factor authentication (MFA)**, select **Assign MFA device**.
   3. Name it `root`, choose **Passkey or security key** or **Authenticator app**, and follow the prompts.

   Do not create access keys for the root user.
3. Add a budget:
   1. Search the console for **Budgets**, then go to **Billing and Cost Management** → **Budgets** → **Create budget**.
   2. Choose **Use a template (simplified)**, then **Monthly cost budget**.
   3. **Budget name:** `linkdean monthly`. **Budgeted amount:** 5. **Email recipients:** your address.
   4. Select **Create budget**.

**Success:** the MFA device appears under **Security credentials**, and the budget appears on **Budgets**.

### 9. Console user

You use this user instead of the root user whenever you open the AWS console. Nothing on your Mac needs AWS keys, because OpenTofu does not manage AWS.

1. Search the console for **IAM**, then go to **Users** → **Create user**.
2. **User name:** `arsam`. Tick **Provide user access to the AWS Management Console**.
   - If AWS asks whether you are providing console access to a person, choose **I want to create an IAM user**.
   - **Console password:** Custom password, from your password manager. Untick **Users must create a new password at next sign-in**.

   Select **Next**.
3. Choose **Attach policies directly**, tick **AdministratorAccess**, select **Next**, then **Create user**.
4. Copy the **Console sign-in URL** shown. It looks like `https://123456789012.signin.aws.amazon.com/console`. Save it with the password.
5. Sign out of the root user, and sign in through that URL as `arsam`.
6. Add MFA: open your user name at the top right, then **Security credentials** → **Assign MFA device**, and follow the prompts as in step 8.

Do not create access keys for this user.

**Success:** you are signed in as `arsam`, and **Security credentials** lists the MFA device.

### 10. Cloudflare token for OpenTofu

OpenTofu uses this token to manage the tunnel and the DNS records it adds. It lives in Infisical, never on disk.

1. In the Cloudflare dashboard, go to **Manage account** → **Account API tokens** → **Create Token**. Next to **Create Custom Token**, select **Get started**.
2. Fill in the form:
   - **Token name:** `linkdean-opentofu`.
   - **Permissions:** three rows:
     - Account · **Cloudflare Tunnel** · **Edit**
     - Zone · **DNS** · **Edit**
     - Zone · **Zone** · **Read**
   - **Zone Resources:** Include · Specific zone · `linkdean.app`.
   - **TTL:** one year, with a calendar reminder.
3. Select **Continue to summary**, then **Create Token**, and copy the token.
4. In Infisical, open the project that holds `/linkdean-public`. In the `prod` environment, create a folder `/linkdean-infra`, and add a secret `CLOUDFLARE_API_TOKEN` there with the copied token.
5. Clear the clipboard: `pbcopy < /dev/null`.

**Success:** the Cloudflare page after creation shows a test command. Run it, and it reports the token as valid. Claude's first `tofu plan` in step 12 then reads your Cloudflare records.

## Phase 3

### 11. State bucket

This holds OpenTofu's state. It keeps old versions, so a bad apply can be undone. The state holds no secrets, because every secret value goes straight into Secret Manager (steps 15 to 19), so a private bucket is all it needs.

```sh
PROJECT_ID=linkdean-prod
gcloud storage buckets create "gs://$PROJECT_ID-tfstate" --project="$PROJECT_ID" \
  --location=europe-west2 --uniform-bucket-level-access --public-access-prevention
gcloud storage buckets update "gs://$PROJECT_ID-tfstate" --versioning
```

**Success:** `gcloud storage buckets describe "gs://$PROJECT_ID-tfstate"` shows `versioning_enabled: true`.

### 12. Running OpenTofu

Each time Claude opens an OpenTofu pull request in `linkdean-infra`:

1. Read the pull request. It states what the plan should show.
2. Run the plan from your clone:

   ```sh
   cd ~/code/Sandbox/linkdean-infra/tofu
   ./bin/tofu init
   ./bin/tofu plan -out=plan.tfplan
   ```

   `./bin/tofu` is a small wrapper Claude adds in Phase 3. It runs `tofu` under `infisical run` with `/linkdean-infra`, so the Cloudflare token is only ever in the process's environment.
3. Compare the last line, `Plan: N to add, N to change, N to destroy`, with the pull request. If anything is destroyed or changed that the pull request does not mention, stop and tell Claude.
4. Apply and clean up:

   ```sh
   ./bin/tofu apply plan.tfplan
   rm plan.tfplan
   ```

**Success:** `Apply complete!`, with the same counts as the plan.

### 13. SES identity for `linkdean.app`

1. In the AWS console, signed in as `arsam`, check that the region menu shows **Europe (London)**. Then search for **Amazon Simple Email Service**.
2. Go to **Configuration** → **Identities** → **Create identity**.
3. Fill in the form:
   - **Identity type:** Domain. **Domain:** `linkdean.app`.
   - Tick **Use a custom MAIL FROM domain**. **MAIL FROM domain:** `bounce`. **Behavior on MX failure:** Use default MAIL FROM domain.
   - **Verifying your domain:** Easy DKIM, with **RSA_2048_BIT**.
4. Select **Create identity**.
5. On the identity's page, find the **Publish DNS records** section. Send Claude the three DKIM CNAME rows, each with its name and value. They are not secret. Claude adds them, with the MAIL FROM records, in the next OpenTofu pull request, which you apply as in step 12.
6. Go to **Configuration** → **Suppression list**. Check that the account-level suppression list is on for bounces and complaints. If it is not, select **Edit** and turn it on.

**Success:** after the apply, the identity shows **Verified** and DKIM shows **Successful**. That is usually minutes, and can take up to 72 hours.

### 14. SES production access

New SES accounts can only send to addresses you have verified. Ask AWS to lift that as soon as step 13's identity shows **Verified**.

1. In the SES console, with the region set to **Europe (London)**, go to **Account dashboard**. In the sandbox notice, follow the link to the setup page, then select **Request production access**.
2. Fill in the form:
   - **Mail type:** Transactional.
   - **Website URL:** `https://linkdean.app`.
   - **Use case description:** paste the text below, with your current user count filled in.
   - **Preferred contact language:** English.
3. Tick the agreement to the AWS Service Terms and Acceptable Use Policy, then select **Submit request**.

```text
Linkdean (https://linkdean.app) is a bookmark manager in public beta. We send only
transactional email to people who signed up through our registration form, which is
protected by Cloudflare Turnstile and requires email verification:

- account email: address verification, password resets, notices when a password,
  passkey or email address changes, and account deletion notices
- invitations to shared folders
- notification emails that users switch on themselves; each has a one-click
  unsubscribe link

We send no marketing or bulk email. We have about [N] users and expect fewer than
1,000 emails a month at first. The SES account-level suppression list is on, so
addresses that bounce or complain are not mailed again, and we watch the bounce and
complaint rates on the reputation page.
```

AWS replies by email, and may ask follow-up questions in the Support Center case.

**Success:** the **Account dashboard** no longer shows the sandbox notice, and shows a daily sending quota instead.

## Phase 4

### Before steps 15 to 19

Paste these into your terminal once per session. They set the project and an Infisical shortcut, and define `put_secret`. It stores a command's output as a new secret version, and refuses empty output or a "not found" result, so a failed lookup never becomes a stored secret.

```sh
set -o pipefail
PROJECT_ID=linkdean-prod
# The same Infisical project ID and server address that render-env-from-infisical.sh uses.
INF_BETA=(--env=prod --path=/linkdean-public --projectId=YOUR_INFISICAL_PROJECT_ID --domain=YOUR_INFISICAL_URL)
INF_INFRA=(--env=prod --path=/linkdean-infra --projectId=YOUR_INFISICAL_PROJECT_ID --domain=YOUR_INFISICAL_URL)

put_secret() {
  local name=$1 value
  shift
  value=$("$@") || { echo "not copied: $name" >&2; return 1; }
  if [ -z "$value" ] || [ "$value" = "*not found*" ]; then
    echo "empty or missing: $name" >&2
    return 1
  fi
  printf '%s' "$value" | gcloud secrets versions add "$name" --data-file=- \
    --project="$PROJECT_ID" >/dev/null && echo "copied: $name"
}
```

Check any secret afterwards without showing its value:

```sh
gcloud secrets versions list NAME --project="$PROJECT_ID"
```

The command lists one enabled version.

### 15. Database user and `DATABASE_URL`

The password goes straight from a random generator into Google Cloud. It never appears on screen or in OpenTofu's state.

```sh
cd ~/code/Sandbox/linkdean-infra/tofu
INSTANCE=$(./bin/tofu output -raw sql_instance_name)
DB_HOST=$(./bin/tofu output -raw sql_private_ip)
PW=$(openssl rand -hex 32)
gcloud sql users create linkdean --instance="$INSTANCE" --password="$PW" --project="$PROJECT_ID" &&
  put_secret DATABASE_URL printf 'postgres://linkdean:%s@%s:5432/linkdean?sslmode=require' "$PW" "$DB_HOST"
# gcloud may write command arguments to its local logs; remove any log file that holds the password.
grep -rlF -- "$PW" ~/.config/gcloud/logs 2>/dev/null | xargs rm -f
unset PW
```

If the user is created but the secret step fails, run the block again with `gcloud sql users set-password` in place of `gcloud sql users create`.

**Success:** `copied: DATABASE_URL`.

### 16. Tunnel token

OpenTofu creates the `linkdean-gcp` tunnel but never reads its token. This command fetches the token with the OpenTofu Cloudflare token, and stores it directly:

```sh
cd ~/code/Sandbox/linkdean-infra/tofu
export CF_ACCOUNT=YOUR_CLOUDFLARE_ACCOUNT_ID
export TUNNEL_ID=$(./bin/tofu output -raw tunnel_id)
put_secret TUNNEL_TOKEN infisical run "${INF_INFRA[@]}" -- sh -c \
  'curl -fsS -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" "https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT/cfd_tunnel/$TUNNEL_ID/token" | jq -er .result'
```

**Success:** `copied: TUNNEL_TOKEN`.

### 17. Meilisearch key

The new search server gets a new key. The index is rebuilt by a reindex, so nothing needs the old one.

```sh
put_secret MEILI_MASTER_KEY openssl rand -hex 32
```

**Success:** `copied: MEILI_MASTER_KEY`.

## Phase 6

### 18. Secrets from Infisical

Claude lists the exact names in the Phase 6 pull request. It takes them from the key names in the guest's `.env`, without reading the values. The likely set is below; remove any you do not use:

```sh
for NAME in SESSION_SECRET TURNSTILE_SITE_KEY TURNSTILE_SECRET_KEY GOOGLE_CLIENT_ID GOOGLE_CLIENT_SECRET; do
  put_secret "$NAME" infisical secrets get "$NAME" "${INF_BETA[@]}" --plain --silent
done
```

`SESSION_SECRET` keeps its current value. Sessions do not depend on it, but it signs the unsubscribe links in emails already sent, and a new value would break those links.

**Success:** one `copied:` line per name, and no `not copied` or `empty` lines.

### 19. SES SMTP login

1. In the SES console (London), go to **SMTP settings**. Check that the endpoint shown is `email-smtp.eu-west-2.amazonaws.com`.
2. Select **Create SMTP credentials**. Name the IAM user `linkdean-ses-smtp`, select **Create user**, then **Download .csv file**.

   The file is a credential: it holds the SMTP password.
3. Copy both values from the file and delete it, but only when both copies worked:

   ```sh
   CSV=~/Downloads/linkdean-ses-smtp_credentials.csv   # the file you just downloaded
   field() { python3 -c 'import csv, sys; sys.stdout.write(list(csv.reader(open(sys.argv[1])))[1][int(sys.argv[2])])' "$CSV" "$1"; }
   put_secret SMTP_USERNAME field 1 && put_secret SMTP_PASSWORD field 2 && rm "$CSV"
   ```

   The columns are the IAM user name, then the SMTP user name, then the SMTP password. Open the file in a text editor if the copy fails, to check the order, but do not paste anything from it.

**Success:** `copied: SMTP_USERNAME`, then `copied: SMTP_PASSWORD`, and the file is gone.

### 20. Cloudflare Access for the rehearsal

This keeps `app-next.linkdean.app` to you alone while it holds a copy of real data.

1. In the Cloudflare dashboard, go to **Zero Trust** → **Access controls** → **Applications** → **Create new application** → **Self-hosted and private** → **Add public hostname**.
2. Enter subdomain `app-next` and choose the domain `linkdean.app`.
3. Under **Access policies**, create a policy:
   - **Name:** `Only me`.
   - **Action:** Allow.
   - **Include:** one rule, selector **Emails**, with your address.
4. As the login method, choose **One-time PIN**. Then select **Create**.

**Success:** in a private window, `https://app-next.linkdean.app` shows Cloudflare's sign-in page, and your emailed code lets you through.

Delete the application after the rehearsal: Claude reminds you in step 8 of the rehearsal.

## Phase 8

### 21. Cutover day

- [ ] At least 3 days ahead, tell users the date and the window.
- [ ] On the day, approve each command Claude shows you before it runs, and run `tofu apply` for the DNS switch yourself (step 12).
- [ ] Run the smoke test in the plan's Phase 8, step 10, as yourself. That includes a passkey sign-in and a password reset email.

## Phase 9

### 22. Retire the homelab beta

Do this only after 14 quiet days on Google Cloud. Each part cannot be undone.

- [ ] Take a final `pg_dump` from the stopped guest and keep it with your personal backups.
- [ ] Delete the `linkdean-public` LXD guest on Nova.
- [ ] In Backblaze, delete the `linkdean-public` bucket and its application keys. Then remove the restic schedule on Nova.
- [ ] In Infisical, delete the `/linkdean-public` folder. Keep `/linkdean-infra`.
- [ ] In Uptime Kuma, remove the public beta's monitors.
- [ ] In the Cloudflare dashboard, under **Zero Trust** → **Networks**, delete the old public-beta tunnel. Its DNS records were switched or removed earlier.
- [ ] In AWS IAM, keep the `linkdean-ses-smtp` user: the server still sends through it.
