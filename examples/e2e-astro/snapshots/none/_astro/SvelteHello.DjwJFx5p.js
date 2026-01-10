import"./legacy.DqdqzNL1.js";import{g as R,p as A,P as I,a as g,b as L,s as T,c as B,D as Y,d as q,l as w,e as y,f as H,i as N,S as U,L as $,h as j,j as C,t as G,k as M,m as W,n as b,r as h,o as E,q as z}from"./render.BPxthj4-.js";let c=!1;function F(r){var t=c;try{return c=!1,[r(),c]}finally{c=t}}function J(r,t,f,i){var a=!w||(f&y)!==0,v=(f&H)!==0,n=i,o=!0,S=()=>(o&&(o=!1,n=i),n),u;{var x=U in r||$ in r;u=R(r,t)?.set??(x&&t in r?e=>r[t]=e:void 0)}var _,m=!1;[_,m]=F(()=>r[t]),_===void 0&&i!==void 0&&(_=S(),u&&(a&&A(),u(_)));var s;if(a?s=()=>{var e=r[t];return e===void 0?S():(o=!0,e)}:s=()=>{var e=r[t];return e!==void 0&&(n=void 0),e===void 0?n:e},a&&(f&I)===0)return s;if(u){var D=r.$$legacy;return(function(e,d){return arguments.length>0?((!a||!d||D||m)&&u(d?s():e),e):s()})}var p=!1,l=q(()=>(p=!1,s()));g(l);var O=B;return(function(e,d){if(arguments.length>0){const P=d?g(l):a&&v?L(e):e;return T(l,P),p=!0,n!==void 0&&(n=P),e}return N&&p||(O.f&Y)!==0?l.v:g(l)})}var K=j(`<p>Hello <span> </span>!</p> <p>We support stripping whitespace in Svelte components.</p> <p>Svelte
    preserves
      consecutive
        spaces
          in
            text
              nodes.</p>`,1);function X(r,t){let f=J(t,"name",8,"Svelte");var i=K(),a=C(i),v=W(b(a)),n=b(v,!0);h(v),E(),h(a),E(4),G(()=>z(n,f())),M(r,i)}export{X as default};
