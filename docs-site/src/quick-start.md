# Quick Start

## 1. Initialize a config

```bash
guildforge init --template minimal
```

This creates `guildforge.yaml` from the `minimal` template:

```yaml
server:
  name: My Server

roles:
  - name: Admin
    color: red
    permissions: [administrator]

channels:
  - name: general
    type: text
    topic: General chat
    permissions:
      read: [everyone]
      write: [Admin]
```

## 2. Validate

```bash
guildforge validate guildforge.yaml
```

## 3. Plan

```bash
guildforge plan guildforge.yaml
```

Output:

```
+ role  role/Admin
+ role  role/@everyone (implied)
+ channel  channel/_top/general

Plan: +3 ~0 -0 =0
```

## 4. Apply

```bash
guildforge apply --auto-approve guildforge.yaml
```

## 5. Verify idempotency

Run apply again — it should be a no-op:

```bash
guildforge apply --auto-approve guildforge.yaml
# Apply complete: +0 ~0 -0 =3
```

## 6. Check for drift

```bash
guildforge doctor
# No drift detected.
```

## 7. Make a change

Edit `guildforge.yaml` to add a channel:

```yaml
channels:
  - name: general
    type: text
  - name: announcements
    type: text
```

Plan and apply:

```bash
guildforge plan guildforge.yaml
# + channel  channel/_top/announcements
# Plan: +1 ~0 -0 =3

guildforge apply --auto-approve guildforge.yaml
# Apply complete: +1 ~0 -0 =3
```

## 8. Destroy

Tear everything down:

```bash
guildforge destroy --auto-approve guildforge.yaml
# Destroy complete: +0 ~0 -4 =0
```
