/*
 * Vidya mock renderer — dev only. Renders a `.dc.html` mock in a plain browser.
 *
 * A mock file has three parts inside <body>:
 *   <x-dc>            the template (with <helmet>, holes, <sc-for>, <sc-if>, on* handlers)
 *   <helmet>          font <link>s + <style>, moved into <head>
 *   <script type="text/x-dc" data-dc-script data-props='...'>  the Component logic
 *
 * This script implements the DCLogic base class, evaluates the mock's Component,
 * resolves the template DSL against component.renderVals(), and mounts the result.
 * No imports, no dependencies. ES2018.
 */
(function () {
  'use strict';

  // ---- Asset blob table (docs/01-MOCK-SPEC.md §4) -------------------------
  var BLOBS = {
    c7724ba7cede6cc54dcf27ecbe1f94fd: '../assets/vidya-horizontal-on-dark.svg',
    '018c620b68aa5806aa7ee412a1ecdefe': '../assets/vidya-horizontal-on-light.svg',
    c7a72d1fc8dc4a2020b3cd64bb5ad940: '../assets/dwaar-mark-on-dark.svg'
  };

  // ---- DCLogic base class (exposed as a global for the mock script) -------
  var currentInstance = null; // the mounted component, used by setState -> rerender

  function DCLogic(props) {
    this.props = props || {};
    this.state = {};
  }
  DCLogic.prototype.setState = function (patch) {
    var next = {};
    var k;
    for (k in this.state) if (Object.prototype.hasOwnProperty.call(this.state, k)) next[k] = this.state[k];
    for (k in patch) if (Object.prototype.hasOwnProperty.call(patch, k)) next[k] = patch[k];
    this.state = next;
    if (currentInstance === this) render();
  };
  DCLogic.prototype.componentWillUnmount = function () {};
  window.DCLogic = DCLogic;

  // ---- Hole resolution ----------------------------------------------------
  // Resolve a dot path (e.g. "a.b.c") against a scope object.
  function resolvePath(path, scope) {
    var parts = path.split('.');
    var val = scope;
    for (var i = 0; i < parts.length; i++) {
      if (val == null) return undefined;
      val = val[parts[i].trim()];
    }
    return val;
  }

  var WHOLE_HOLE = /^\s*\{\{\s*([^}]+?)\s*\}\}\s*$/; // the entire string is one hole
  var HOLE = /\{\{\s*([^}]+?)\s*\}\}/g;             // any hole (for embedded text)

  // If the string is exactly one hole, return the raw resolved value (may be a
  // function, boolean, object). Otherwise interpolate holes as strings.
  function resolveValue(str, scope) {
    var m = WHOLE_HOLE.exec(str);
    if (m) return resolvePath(m[1], scope);
    return str.replace(HOLE, function (_, path) {
      var v = resolvePath(path, scope);
      return v == null ? '' : String(v);
    });
  }

  function hasHole(str) {
    return str.indexOf('{{') !== -1;
  }

  // ---- Attribute handling -------------------------------------------------
  var EVENT_ATTRS = { onClick: 'click', onChange: 'change', onInput: 'input' };
  var BOOL_PROPS = { disabled: 1, checked: 1, readonly: 1, readOnly: 1 };
  var ARIA_BOOL = { 'aria-checked': 1, 'aria-pressed': 1, 'aria-selected': 1, 'aria-expanded': 1 };

  // Apply one attribute (already read as name/value strings) to a real element,
  // resolving holes against scope.
  function applyAttr(el, name, rawValue, scope) {
    // Event handlers: onClick/onChange/onInput -> resolved value is a function.
    if (EVENT_ATTRS.hasOwnProperty(name)) {
      var fn = resolveValue(rawValue, scope);
      if (typeof fn === 'function') el.addEventListener(EVENT_ATTRS[name], fn);
      return; // never leave on* in the DOM
    }

    var resolved = hasHole(rawValue) ? resolveValue(rawValue, scope) : rawValue;

    // Boolean properties: disabled="{{x}}" -> set/remove from truthiness.
    if (BOOL_PROPS.hasOwnProperty(name)) {
      var prop = name === 'readonly' ? 'readOnly' : name;
      if (resolved) {
        el[prop] = true;
        if (name === 'disabled') el.setAttribute('disabled', '');
      } else {
        el[prop] = false;
        el.removeAttribute(name === 'readOnly' ? 'readonly' : name);
      }
      return;
    }

    // ARIA booleans: render the string "true" / "false".
    if (ARIA_BOOL.hasOwnProperty(name)) {
      el.setAttribute(name, resolved ? 'true' : 'false');
      return;
    }

    // src="/_blob/<id>" -> ../assets/<file>
    if (name === 'src' && typeof resolved === 'string') {
      var bm = /^\/_blob\/([a-f0-9]+)$/i.exec(resolved);
      if (bm && BLOBS[bm[1]]) resolved = BLOBS[bm[1]];
    }

    if (resolved == null || resolved === false) return;
    el.setAttribute(name, String(resolved));

    // `value` on inputs: also set the property so the field shows the value and
    // re-renders update the live element.
    if (name === 'value' && ('value' in el)) el.value = String(resolved);
  }

  // ---- Template -> DOM ----------------------------------------------------
  // Render a source node (from the parsed template) into the target parent,
  // resolving DSL against `scope`. Handles <sc-for>, <sc-if>, holes, on*.
  function renderNode(srcNode, targetParent, scope) {
    // Text node: interpolate holes.
    if (srcNode.nodeType === 3) {
      var text = srcNode.nodeValue;
      var out = hasHole(text) ? resolveValue(text, scope) : text;
      targetParent.appendChild(document.createTextNode(out == null ? '' : String(out)));
      return;
    }
    // Comments and anything non-element: skip.
    if (srcNode.nodeType !== 1) return;

    var tag = srcNode.tagName.toLowerCase();

    // <sc-for list="{{items}}" as="x"> — repeat inner markup per item.
    if (tag === 'sc-for') {
      var listExpr = srcNode.getAttribute('list') || '';
      var asName = srcNode.getAttribute('as') || 'item';
      var list = resolveValue(listExpr, scope);
      if (Array.isArray(list)) {
        for (var i = 0; i < list.length; i++) {
          var childScope = makeScope(scope);
          childScope[asName] = list[i];
          childScope.$index = i;
          renderChildren(srcNode, targetParent, childScope);
        }
      }
      return;
    }

    // <sc-if value="{{cond}}"> — include inner markup only if truthy.
    if (tag === 'sc-if') {
      var cond = resolveValue(srcNode.getAttribute('value') || '', scope);
      if (cond) renderChildren(srcNode, targetParent, scope);
      return;
    }

    // Ordinary element: clone shallow, apply attributes, recurse into children.
    var el = document.createElement(srcNode.tagName.toLowerCase());
    var attrs = srcNode.attributes;
    for (var a = 0; a < attrs.length; a++) {
      applyAttr(el, attrs[a].name, attrs[a].value, scope);
    }
    targetParent.appendChild(el);
    renderChildren(srcNode, el, scope);
  }

  function renderChildren(srcNode, targetParent, scope) {
    var kids = srcNode.childNodes;
    for (var i = 0; i < kids.length; i++) renderNode(kids[i], targetParent, scope);
  }

  // Shallow-inherit scope so loop variables shadow without mutating the parent.
  function makeScope(parent) {
    var s = {};
    for (var k in parent) if (Object.prototype.hasOwnProperty.call(parent, k)) s[k] = parent[k];
    return s;
  }

  // ---- Mount / render loop ------------------------------------------------
  var templateNode = null; // the <x-dc>'s content template (a cloned fragment source)
  var mountEl = null;      // the container we render into

  function render() {
    if (!currentInstance || !templateNode || !mountEl) return;

    // Preserve focus + caret before we tear down the DOM.
    var active = document.activeElement;
    var focusInfo = null;
    if (active && active.id && mountEl.contains(active) &&
        (active.tagName === 'INPUT' || active.tagName === 'TEXTAREA')) {
      focusInfo = { id: active.id };
      try {
        focusInfo.start = active.selectionStart;
        focusInfo.end = active.selectionEnd;
      } catch (e) { /* some input types disallow selection access */ }
    }

    var scope = currentInstance.renderVals() || {};

    // Full re-render: clear and rebuild from the template.
    while (mountEl.firstChild) mountEl.removeChild(mountEl.firstChild);
    renderChildren(templateNode, mountEl, scope);

    // Restore focus + caret.
    if (focusInfo) {
      var next = document.getElementById(focusInfo.id);
      if (next && (next.tagName === 'INPUT' || next.tagName === 'TEXTAREA')) {
        next.focus();
        if (focusInfo.start != null) {
          try { next.setSelectionRange(focusInfo.start, focusInfo.end); } catch (e2) { /* ignore */ }
        }
      }
    }
  }

  // ---- Boot ---------------------------------------------------------------
  function boot() {
    var xdc = document.querySelector('x-dc');
    var scriptTag = document.querySelector('script[type="text/x-dc"]');
    if (!xdc || !scriptTag) return;

    // Move <helmet> children (font links, <style>) into <head>.
    var helmet = xdc.querySelector('helmet') || document.querySelector('helmet');
    if (helmet) {
      var head = document.head || document.getElementsByTagName('head')[0];
      // Move real element children (skip text/whitespace).
      var hk = Array.prototype.slice.call(helmet.childNodes);
      for (var i = 0; i < hk.length; i++) {
        if (hk[i].nodeType === 1) head.appendChild(hk[i]);
      }
      if (helmet.parentNode) helmet.parentNode.removeChild(helmet);
    }

    // Read props: use each prop's `default`; honor ?name= URL overrides.
    var props = {};
    try {
      var raw = scriptTag.getAttribute('data-props');
      if (raw) {
        var spec = JSON.parse(raw);
        var params = new URLSearchParams(window.location.search);
        for (var key in spec) {
          if (!Object.prototype.hasOwnProperty.call(spec, key)) continue;
          if (key.charAt(0) === '$') continue; // $preview etc. — layout is fixed in the template
          var def = spec[key];
          var value = def && typeof def === 'object' && 'default' in def ? def.default : def;
          if (params.has(key)) value = params.get(key);
          props[key] = value;
        }
      }
    } catch (e) { /* fall back to empty props */ }

    // Define the mock's Component via new Function so `class Component extends DCLogic`
    // is in scope with DCLogic injected.
    var Component;
    try {
      Component = new Function('DCLogic', scriptTag.textContent + '\n; return Component;')(DCLogic);
    } catch (e) {
      console.error('[support.js] failed to evaluate mock script:', e);
      return;
    }

    // Capture the template: the <x-dc>'s children (helmet already removed) become
    // the render source. Keep them detached so re-renders always start clean.
    templateNode = document.createElement('template').content || document.createDocumentFragment();
    var tk = Array.prototype.slice.call(xdc.childNodes);
    for (var j = 0; j < tk.length; j++) templateNode.appendChild(tk[j]);

    // Create the mount point where <x-dc> was, then remove the raw <x-dc>.
    mountEl = document.createElement('div');
    if (xdc.parentNode) {
      xdc.parentNode.insertBefore(mountEl, xdc);
      xdc.parentNode.removeChild(xdc);
    } else {
      document.body.appendChild(mountEl);
    }

    // Instantiate + first render.
    currentInstance = new Component(props);
    render();

    // Clean up timers if the page goes away.
    window.addEventListener('beforeunload', function () {
      if (currentInstance && typeof currentInstance.componentWillUnmount === 'function') {
        try { currentInstance.componentWillUnmount(); } catch (e3) { /* ignore */ }
      }
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot);
  } else {
    boot();
  }
})();
