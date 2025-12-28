import"./legacy.DqdqzNL1.js";import{g as w,p as x,P as A,a as p,b as I,s as L,c as T,D as B,d as Y,l as q,e as y,f as C,i as H,S as N,L as U,h as $,j,t as G,k as M,m as z,n as b,r as P,o as E,q as F}from"./render.BPxthj4-.js";let o=!1;function J(a){var r=o;try{return o=!1,[a(),o]}finally{o=r}}function K(a,r,f,i){var t=!q||(f&y)!==0,d=(f&C)!==0,n=i,c=!0,S=()=>(c&&(c=!1,n=i),n),u;{var D=N in a||U in a;u=w(a,r)?.set??(D&&r in a?e=>a[r]=e:void 0)}var _,h=!1;[_,h]=J(()=>a[r]),_===void 0&&i!==void 0&&(_=S(),u&&(t&&x(),u(_)));var s;if(t?s=()=>{var e=a[r];return e===void 0?S():(c=!0,e)}:s=()=>{var e=a[r];return e!==void 0&&(n=void 0),e===void 0?n:e},t&&(f&A)===0)return s;if(u){var O=a.$$legacy;return(function(e,v){return arguments.length>0?((!t||!v||O||h)&&u(v?s():e),e):s()})}var g=!1,l=Y(()=>(g=!1,s()));p(l);var R=T;return(function(e,v){if(arguments.length>0){const m=v?p(l):t&&d?I(e):e;return L(l,m),g=!0,n!==void 0&&(n=m),e}return H&&g||(R.f&B)!==0?l.v:p(l)})}var Q=$(`<p>Hello<span> </span>!</p><p>Svelte has own whitespace handling so Astro's strip-whitespace feature should
  not affect it.</p><p>Consecutive
    spaces
      and
        new
          lines
            should
              be
                preserved.</p>`,1);function X(a,r){let f=K(r,"name",8,"Svelte");var i=Q(),t=j(i),d=z(b(t)),n=b(d,!0);P(d),E(),P(t),E(2),G(()=>F(n,f())),M(a,i)}export{X as default};
