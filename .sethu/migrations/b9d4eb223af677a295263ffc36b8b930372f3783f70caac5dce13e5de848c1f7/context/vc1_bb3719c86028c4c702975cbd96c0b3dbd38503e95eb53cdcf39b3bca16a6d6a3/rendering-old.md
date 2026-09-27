# Immich

Immich API

API Version: 1.116.2

## Endpoints

### updateConfig {#put-system-config}
**Operation:** PUT /system-config
**Operation ID:** `updateConfig`

#### Parameters
| Name | In | Required | Description |
|------|----|---------:|-------------|
| `requestBody` | body | Yes | - |

#### Responses
| Code | Type | Description |
|------|------|-------------|
| 200 | application/json |  |

---

## Spec Hygiene Report

**1 endpoint** across **1 service**

| Check | Count |
|-------|------:|
| Missing description | 1 |
| Missing operationId | 0 |
| No responses documented | 0 |
| Deprecated | 0 |
| Untagged (no service tag) | 0 |
| Duplicate operationIds | 0 |
| Parameters without description | 1 |

### Missing description (1)
- `PUT /system-config`

### Parameters without description (1)
- `PUT /system-config` — `requestBody`
