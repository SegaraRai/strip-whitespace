import"./legacy.XFWhk1-4.js";import{g as R,p as A,P as I,a as g,b as L,s as T,c as B,D as Y,l as q,d as w,e as y,f as H,i as N,S as U,L as $,h as j,t as C,j as G,k as M,m as W,n as b,r as h,o as E,q as z}from"./render.OUvbLN8V.js";let c=!1;function F(t){var r=c;try{return c=!1,[t(),c]}finally{c=r}}function J(t,r,f,i){var a=!q||(f&w)!==0,_=(f&y)!==0,n=i,o=!0,S=()=>(o&&(o=!1,n=i),n);let u;{var x=U in t||$ in t;u=R(t,r)?.set??(x&&r in t?e=>t[r]=e:void 0)}var d,m=!1;[d,m]=F(()=>t[r]),d===void 0&&i!==void 0&&(d=S(),u&&(a&&A(),u(d)));var s;if(a?s=()=>{var e=t[r];return e===void 0?S():(o=!0,e)}:s=()=>{var e=t[r];return e!==void 0&&(n=void 0),e===void 0?n:e},a&&(f&I)===0)return s;if(u){var D=t.$$legacy;return(function(e,v){return arguments.length>0?((!a||!v||D||m)&&u(v?s():e),e):s()})}var p=!1,l=H(()=>(p=!1,s()));g(l);var O=B;return(function(e,v){if(arguments.length>0){const P=v?g(l):a&&_?L(e):e;return T(l,P),p=!0,n!==void 0&&(n=P),e}return N&&p||(O.f&Y)!==0?l.v:g(l)})}var K=M(`<p>Hello<span> </span>!</p><p>We support stripping whitespace in Svelte components.</p><p>Svelte
    preserves
      consecutive
        spaces
          in
            text
              nodes.</p>`,1);function X(t,r){let f=J(r,"name",8,"Svelte");var i=K(),a=j(i),_=W(b(a)),n=b(_,!0);h(_),E(),h(a),E(2),C(()=>z(n,f())),G(t,i)}export{X as default};
