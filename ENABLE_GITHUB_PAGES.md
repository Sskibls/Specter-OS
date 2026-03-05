# 🚀 Enable GitHub Pages - Quick Guide

## The Error You're Seeing

```
Error: Get Pages site failed. Please verify that the repository has Pages enabled
```

This means GitHub Pages needs to be manually enabled first.

---

## ✅ Solution 1: Enable Pages Manually (Recommended)

### Step 1: Go to Settings
```
https://github.com/Sskibls/Specteros-OS/settings/pages
```

### Step 2: Configure Pages Source

**Under "Build and deployment":**

1. **Source:** Select **"Deploy from a branch"**

2. **Branch:** Select:
   - Branch: `main`
   - Folder: `/website` (or `root` if `/website` not available)

3. Click **"Save"**

### Step 3: Wait for Deployment

GitHub will build and deploy your site in 1-2 minutes.

### Step 4: Visit Your Site

```
https://sskibls.github.io/Specteros-OS/
```

---

## ✅ Solution 2: Use GitHub Actions (After Manual Enable)

Once Pages is enabled manually, the workflow will work automatically.

The workflow file `.github/workflows/deploy-pages.yml` will:
- Trigger on every push to `website/` folder
- Build the website
- Deploy to GitHub Pages

---

## ✅ Solution 3: Alternative - Use Netlify (Easier)

If GitHub Pages continues to have issues:

### Deploy to Netlify (Free):

1. Go to https://netlify.com
2. Sign in with GitHub
3. Click "Add new site" → "Import an existing project"
4. Connect to `Sskibls/Specteros-OS`
5. Build settings:
   - **Base directory:** `website`
   - **Publish directory:** `.` (root)
6. Click "Deploy"

Your site will be live at:
```
https://specteros-os.netlify.app
```

---

## 🔍 Troubleshooting

### Still Getting Errors?

1. **Check if Pages is enabled:**
   - Go to Settings → Pages
   - You should see "Your site is live at..."

2. **Check workflow permissions:**
   - Settings → Actions → General
   - Ensure "Read and write permissions" is enabled

3. **Check branch protection:**
   - Settings → Branches
   - Ensure `main` branch allows Actions

4. **Try disabling and re-enabling Pages:**
   - Settings → Pages → Disable
   - Wait 30 seconds
   - Enable again

---

## 📊 Check Deployment Status

Visit:
```
https://github.com/Sskibls/Specteros-OS/actions
```

You should see the "Deploy to GitHub Pages" workflow running.

---

## ✨ What You'll Get

Once deployed, your professional SpecterOS website will be live with:

- ✅ Modern dark theme
- ✅ Terminal-style hero section
- ✅ Feature comparison table
- ✅ Responsive design
- ✅ Professional typography
- ✅ Smooth animations

---

**TL;DR: Go to Settings → Pages → Select "Deploy from a branch" → Choose main branch → Save**

Then wait 2 minutes and visit: `https://sskibls.github.io/Specteros-OS/`
