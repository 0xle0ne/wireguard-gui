## Description

Fixes a CSS class typo in the disconnected state icon that prevents the animation from working properly.

## Problem

In `app/page.tsx` line 96, there's a missing space between two CSS classes:
```tsx
<Unlock className="animate-pulsemb-2 size-16 text-red-500" />
```

This causes:
- The pulse animation to not apply (Tailwind can't parse `animate-pulsemb-2`)
- Incorrect spacing below the icon (missing `mb-2`)

## Solution

Added the missing space between classes:
```tsx
<Unlock className="animate-pulse mb-2 size-16 text-red-500" />
```

## Testing

- [x] Verified animation works in disconnected state
- [x] Verified proper bottom margin spacing
- [x] Tested in both light and dark themes
- [x] No console errors

## Screenshots

### Before
- No pulse animation on unlock icon when disconnected
- Incorrect spacing

### After
- Smooth pulse animation on unlock icon
- Proper spacing below icon

## Type of Change

- [x] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Checklist

- [x] Code follows the project's style guidelines
- [x] Changes have been tested locally
- [x] No new warnings or errors introduced
- [x] Commit message follows conventional commits format

