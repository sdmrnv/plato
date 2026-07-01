cat src/App.svelte                
<!-- eslint-disable svelte/no-at-html-tags -->
<script lang="ts">
  const TITLE_TEXT = "PLATO";
  const MENU_LABEL_DIALOGI = "Диалоги";
  const MENU_LABEL_EIDOS = "О проекте";      
  const MENU_LABEL_METRICS = "Метрики";
  const SELECTED_MACHINERY_NAME = "машинерия";
  const ID_DIALOGI = 1023;
  const ID_ABOUT = 729;

  let dialogueIds: string[] = [];

  interface Route {
    view: 'esse' | 'metrics';
    id: number | null;
  }

  function parseRoute(hash: string): Route {
    if (!hash) {
      return { view: 'esse', id: ID_DIALOGI };
    }
    if (hash.startsWith('/esse')) {
      const parts = hash.split('/');
      const targetId = Number(parts[2]);
      const finalId = !isNaN(targetId) && parts[2] !== undefined ? targetId : ID_ABOUT;
      return { view: 'esse', id: finalId };
    }
    if (hash.startsWith('/metrics')) return { view: 'metrics', id: null };
    
    window.location.hash = `/esse/${ID_DIALOGI}`;
    return { view: 'esse', id: ID_DIALOGI };
  }

  let isLoading = $state(false);
  let fetchedHtmlContent = $state("");

  let currentRoute = $state<Route>(parseRoute(window.location.hash.substring(1)));
  let view = $state<'esse' | 'metrics'>(currentRoute.view);
  let activeId = $state<number | null>(currentRoute.id);

  let abortController: AbortController;

  async function fetchCorePayload(target: number | string | null, nextView = view, nextId = activeId) {
    if (target === null) return;
    const endpoint = typeof target === 'number' ? `/esse/${target}` : target;
    isLoading = true;
    try {
      const res = await fetch(endpoint, { signal: abortController.signal });
      if (!res.ok) throw new Error(`HTTP Core Failure: ${res.status}`);

      const content = await res.text();

      fetchedHtmlContent = content;

      /* BUILDING THE SLOT NUMBER MAP */
      if (nextId === ID_DIALOGI) {
        const doc = new DOMParser().parseFromString(content, 'text/html');
        dialogueIds = Array.from(doc.querySelectorAll('.plato-signature'))
          .map(a => a.getAttribute('href')?.split('/').pop() || '')
          .filter(id => id !== ''); /* Filter out empty strings */
      }

      view = nextView;
      activeId = nextId;
    } catch (error: unknown) {
      if ((error as Error).name === 'AbortError') return;
      const errMsg = error instanceof Error ? error.message : "Unexpected failure";
      fetchedHtmlContent = `<div style="color: #cc0000; font-weight: bold; padding: 20px 0;">PLATO core networking error: ${errMsg}</div>`;
      view = nextView;
      activeId = nextId;
    } finally {
      isLoading = false;
    }
  }

  $effect(() => {
    abortController?.abort();
    abortController = new AbortController();

    const targetView = currentRoute.view;
    const targetId = currentRoute.id;

    if (targetView === 'esse') {
      void fetchCorePayload(targetId, targetView, targetId); 
    } else if (targetView === 'metrics') {
      void fetchCorePayload('/metrics', targetView, targetId);
    } 
  });


  function handleLogoNavigation(e: MouseEvent) {
    if (view === 'esse' && activeId !== ID_DIALOGI) {
      e.preventDefault();
      
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
      const clickX = e.clientX - rect.left;
      const elementWidth = rect.width;
      const ratio = clickX / elementWidth;
      
      const currentIndex = dialogueIds.indexOf(String(activeId));
      if (currentIndex === -1) return;

      // VECTOR PL (Previous Letter): Left 40% of the word
      if (ratio < 0.40) {
        if (currentIndex > 0) {
          const prevEsseId = Number(dialogueIds[currentIndex - 1]);
          void fetchCorePayload(prevEsseId, 'esse', prevEsseId);
        }
      } 
      // VECTOR TO (Next Letter): Right 40% of the word (from 60% and above)
      else if (ratio > 0.60) {
        if (currentIndex < dialogueIds.length - 1) {
          const nextEsseId = Number(dialogueIds[currentIndex + 1]);
          void fetchCorePayload(nextEsseId, 'esse', nextEsseId);
        }
      }
    }
  }


</script>

<svelte:window onhashchange={() => {
  const next = parseRoute(window.location.hash.substring(1));
  // Fetch the new route, but DO NOT switch the screen yet!
  void fetchCorePayload(next.view === 'esse' ? next.id : '/metrics', next.view, next.id);
}} />

<div class="root-stack" data-view={view} data-id={activeId}>
  <header class="app-header">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <a 
      href={view === 'esse' && activeId === ID_DIALOGI ? '#plato-bottom-anchor' : 'javascript:void(0)'} 
      class="header-logo-link"
    >
      <span onclick={handleLogoNavigation}>{TITLE_TEXT}</span>
    </a>
  </header>

  <div class="workspace-split">
    <aside class="nexus-menu">
      <div class="admin-portal-gate">
        <a href="/admin" class="admin-link">   
         {SELECTED_MACHINERY_NAME}
        </a>
   </div>
      <nav>
        <ul class="nav-list">
          <li>
            <a href="#/esse/{ID_DIALOGI}" class="nav-link" class:active={activeId === ID_DIALOGI}>
              <span class="full-text">{MENU_LABEL_DIALOGI}</span>
              <span class="short-text">Д</span>
            </a>
          </li>
          <li>
            <a href="#/esse/{ID_ABOUT}" class="nav-link" class:active={activeId === ID_ABOUT}>
              <span class="full-text">{MENU_LABEL_EIDOS}</span>
              <span class="short-text">О</span>
            </a>
          </li>
          <li>
            <a href="#/metrics" class="nav-link" class:active={view === 'metrics'}>
              <span class="full-text">{MENU_LABEL_METRICS}</span>
              <span class="short-text">М</span>
            </a>
          </li>
        </ul>
      </nav>
    </aside>
    <div></div>

    <main class="content-viewport">
      <article class="plato-paper-card" data-loading={isLoading} hidden={view !== 'esse'}>
        <div onclick={(e) => {
          if ((e.target as HTMLElement).closest('.plato-journal-footer')) {
            e.preventDefault();
            window.scrollTo({ top: 0, behavior: 'smooth' });
            return;
          }
          const href = (e.target as HTMLElement).closest('a')?.getAttribute('href');
          if (href?.startsWith('/esse/') && !(e.target as HTMLElement).closest('a')?.hasAttribute('target')) { e.preventDefault(); window.location.hash = href; }
        }} role="presentation">
          {@html fetchedHtmlContent}
        </div>
      </article>
      <!-- TELEMETRY ZONE -->
      <pre class="telemetry-terminal" data-loading={isLoading} hidden={view !== 'metrics'}>
    ____  __    ___  ______ ____     ______ ____   ____   ______
   / __ \/ /   /   |/_  __// __ \   / ____// __ \ / __ \ / ____/
  / /_/ / /   / /| | / /  / / / /  / /    / / / // /_/ // __/   
 / ____/ /___/ ___ |/ /  / /_/ /  / /___ / /_/ // _, _// /___   
/_/   /_____/_/  |_/_/   \____/   \____/ \____//_/ |_|/_____/   
                                                                
{@html fetchedHtmlContent}
      </pre>
    <div id="plato-bottom-anchor"></div>
    </main>

  </div>
</div>


<style>

  @font-face {
    font-family: 'Inconsolata';
    src: url('Inconsolata-Regular.v1.woff2') format('woff2'),
         url('Inconsolata-Regular.v1.woff') format('woff');
    font-weight: normal;
    font-style: normal;
  }

  :global(body) {
    margin: 0;
    padding: 0;
    background-color: #717681;
    overflow-x: hidden;
  }

  .root-stack {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    font-family: sans-serif;
    min-width: 1600px;
    background-color: #717681;
    --menu-active: #4e73a2;
    --menu-inactive: #bcbcc2;
    --menu-admin: #4f4f5a;
    --menu-active-bg: #2a2a30;
  }

  .app-header {
    display: flex;
    align-items: center;
    border-top: 1px solid #a2b2c6;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.4);
    height: 100px;
    background: #1e222b;
  }

  .workspace-split {
    display: grid;
    grid-template-columns: 300px 40px 1fr 340px;
    flex: 1;
    min-height: 0;
    width: 100%;
  }

  .nexus-menu {
    padding: 20px;
    box-sizing: border-box;
    border-right: 1px solid #1c1c1f;
    /*border-top: 2px solid #202e3f;*/
    display: flex;
    flex-direction: column;
    background: #121214;
  }

  .admin-portal-gate {
    margin-bottom: 30px;
    border-bottom: 1px solid #1c1c1f;
    padding-bottom: 15px;
  }

  .admin-link {
    display: inline-block;
    text-decoration: none;
    font-size: 14px;
    font-weight: bold;
    transition: color 0.1s ease;
    color: var(--menu-admin);
  }
  
  .admin-link:hover {
    color: #ffffff !important;
  }

  .nav-list {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .nav-list li { margin-bottom: 8px; }

  .nav-link {
    display: block;
    padding: 12px 16px;
    text-decoration: none;
    font-size: 17px;
    font-weight: bold;
    border-radius: 6px;
    transition: all 0.1s ease;
    color: var(--menu-inactive);
    background: transparent;
  }

  .content-viewport {
    padding: 40px;
    display: flex;
    flex-direction: column;
    min-width: 930px;
    box-sizing: border-box;
    max-width: 1200px;
    width: 100%;

  }

  .plato-paper-card {
    width: 100%;
    padding: 40px; 
    box-sizing: border-box;
    box-shadow: 0 15px 40px rgba(0, 0, 0, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.2); 
    font-family: Georgia, serif; 
    border-style: solid;
    background: #e2e8f0; 
    border-width: 1px;
    border-color: #94a3b8;
    white-space: pre-line; 
    color: rgb(15, 23, 42);
    font-size: 17px;
    line-height: 27.2px; 
    text-wrap-mode: wrap;
    white-space-collapse: preserve;
    word-break: break-word;
  }

  .telemetry-terminal {
    color: #00ff66;
    background: #121214;
    border: 0px solid #00ff66;
    border-radius: 0px;
    padding: 40px;
    font-family: monospace;
    font-size: 13px;
    line-height: 1.5;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5), inset 0 2px 8px rgba(0, 0, 0, 0.8);
    overflow-x: auto;
    margin: 0;
    width: 100%;
    box-sizing: border-box;
  }

  .plato-paper-card :global(.plato-journal-footer) {
    border-top: 1px solid rgba(15, 23, 42, 0.15);
    margin-top: 40px;
    padding-top: 16px;
    font-family: monospace;
    font-size: 12px;
    color: #334155;
    text-align: right;
  }

  :global(.correspondence-row) {
    border-bottom: 1px dashed rgba(15, 23, 42, 0.15);
    padding: 10px 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  :global(.correspondence-row:last-child) {
    border-bottom: none;
  }

  :global(.plato-journal-meta) {
    font-family: monospace;
    font-size: 11px;
    color: #475569;
    margin-bottom: 1px;
  }

  :global(.plato-journal-title) {
    font-size: 16px !important;
    line-height: 1.3 !important;
    margin: 0 0 5px 0 !important;
    color: #000000 !important;
    text-align: left !important;
    text-decoration: underline;
    text-decoration-style: dashed;
    text-underline-offset: 3px;
  }

  :global(.plato-journal-preview) {
    margin: 0;
    font-size: 15px;
    line-height: 1.4;
    font-style: italic;
    color: #334155;
  }


  :global(.correspondence-row:has(.admin-directive) .plato-journal-preview) {
    font-family: 'Inconsolata', monospace;   
    font-style: normal;
    font-size: 14px;
    color: #64748b;
  }

  :global(.plato-article-card a.admin-directive) {
    font-family: 'Inconsolata', monospace;
    font-weight: normal;
    color: #64748b;
    text-decoration: underline;
    text-decoration-style: dashed;
    text-underline-offset: 3px;
  }

  :global(.correspondence-row:has(.admin-directive)) {
    background-color: rgba(15, 23, 42, 0.05);
    padding: 12px 16px !important;
    border-radius: 6px;
    margin: 6px 0;
  }

  .plato-paper-card :global(.plato-admin-letter) {
        font-family: 'Inconsolata', monospace;
  }

  .plato-paper-card :global(.plato-admin-letter .plato-title) {
    font-family: 'Inconsolata', monospace !important;
    font-weight: 600;
  }

  .plato-paper-card:has(:global(.plato-admin-letter)) {
    background-color: #dbe1e8;
  }

  .nav-link.active {
    color: var(--menu-active) !important;
    background: var(--menu-active-bg) !important;
  }

  .short-text { display: none; }

  .app-header span {
    color: #ffffff;
    font-size: 36px;
    font-weight: bold;
    letter-spacing: 1px;
    text-shadow: 0 -1px 1px rgba(0, 0, 0, 0.6);
    margin-left: 400px;
    opacity: 0.75;
  }

  :global(.plato-letter-card .plato-footer),
  :global(.plato-article-card .plato-footer),
  :global(.plato-profile-card .plato-footer) {
    border-top: 1px solid rgba(0, 0, 0, 0.08);    
    margin-top: 30px;               
    padding-top: 15px;              
    text-align: right;              
    font-size: 15px;
    color: #334155; 
  }

  :global(.plato-letter-card .plato-signature),
  :global(.plato-article-card .plato-signature),
  :global(.plato-profile-card .plato-signature),
  :global(.plato-article-card a),
  :global(.plato-profile-card a) {
    color: var(--menu-active);      
    text-decoration: none;          
    font-weight: bold;              
  }

  :global(.plato-letter-card .plato-signature:hover),
  :global(.plato-article-card .plato-signature:hover),
  :global(.plato-profile-card .plato-signature:hover),
  :global(.plato-article-card a:hover),
  :global(.plato-profile-card a:hover) {
    color: #3b5984;                 
  }

  :global(.plato-title) {
    font-family: "Georgia", serif;
    color: rgb(15, 23, 42);
    font-size: 32px !important;
    font-weight: 400 !important;  
    line-height: 51.2px !important;    
    margin-top: 0px !important;
    margin-bottom: 12px !important;
    margin-left: 0px !important;
    margin-right: 0px !important;
  }

 :global(html, body) {
	text-rendering: optimizeLegibility;
	-moz-osx-font-smoothing: grayscale;
	-webkit-font-smoothing: antialiased;
 }

 .plato-paper-card[data-loading="true"] > div,
  .telemetry-terminal[data-loading="true"] {
    opacity: 0.6;
    transition: opacity 0.1s ease;
  }

  :global(html) {
    scroll-behavior: smooth;
  }

  .header-logo-link {
    display: inline-block;
    text-decoration: none;
    color: inherit;
  }

  [hidden] { display: none !important; }

  @media screen and (max-width: 1024px) {
    .root-stack { min-width: 100%; width: 100%; }
    .app-header span { margin-left: 47px; opacity: 0.5; font-size: 22px;}
    .workspace-split { grid-template-columns: 35px 0px 1fr 0px; }
    .nexus-menu { padding: 10px 5px; align-items: center; }
    .admin-portal-gate { display: none; }
    .nav-link { padding: 12px 0; text-align: center; }
    .full-text { display: none; }
    .short-text { display: inline; }
    .content-viewport { padding: 0px 0px 0px 0px; min-width: 0; }
    .telemetry-terminal {
      font-size: 7px; 
      letter-spacing: -0.6px; 
      padding: 4px 8px;
    }
    .app-header {height: 50px;}
    .plato-paper-card { 
      padding: 20px; 
      font-size: 15px;
      line-height: 24px;
      text-align: justify;
      -webkit-hyphens: auto;
      hyphens: auto;
      letter-spacing: -0.3px;
      word-spacing: -1px;  
    }
    .plato-paper-card :global(.plato-title) {
      font-size: 22px !important;
      line-height: 30px !important;
      text-align: left !important;
    }
   .plato-paper-card :global(.plato-admin-letter),
   .plato-paper-card :global(.plato-admin-letter *) {
     font-family: 'Inconsolata', monospace !important;
    }

  }

</style>