import {ApiError,fail} from './common.js';
import {authRoutes} from './auth.js';
import {captureRoutes,cleanup} from './captures.js';

export default {
 async fetch(request,env) {
  let response;
  try {
   const url=new URL(request.url);
   if(request.method==='OPTIONS') {
    if(request.headers.get('Origin')!==env.SITE_ORIGIN)fail('forbidden',403);
    response=new Response(null,{status:204,headers:{'Access-Control-Allow-Methods':'GET, POST, PUT, DELETE, OPTIONS','Access-Control-Allow-Headers':'Authorization, Content-Type, X-CSRF-Token','Access-Control-Max-Age':'600'}});
   }else response=await authRoutes(request,env,url)||await captureRoutes(request,env,url);
   if(!response)fail('not_found',404);
  }catch(e){response=Response.json({error:{code:e instanceof ApiError?e.code:'internal_error',message:e instanceof ApiError?e.message:'Request failed'}},{status:e instanceof ApiError?e.status:500});}
  const headers=new Headers(response.headers);
  headers.set('Cache-Control','no-store');headers.set('X-Content-Type-Options','nosniff');headers.set('Referrer-Policy','no-referrer');headers.set('Vary','Origin');
  if(request.headers.get('Origin')===env.SITE_ORIGIN){headers.set('Access-Control-Allow-Origin',env.SITE_ORIGIN);headers.set('Access-Control-Allow-Credentials','true');}
  return new Response(response.body,{status:response.status,headers});
 },
 async scheduled(_controller,env) {await cleanup(env);}
};
