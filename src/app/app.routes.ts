import { BrowserModule, Title } from '@angular/platform-browser';
import { RouterModule, Routes } from '@angular/router';
import { NgModule } from '@angular/core';

// import { HashLocationStrategy, LocationStrategy } from '@angular/common';

import { Docs_T01_IntoComponent } from './docs/t01-starter/into-getting-starter/into-getting-starter.component';
import { Docs_T02_IntoComponent } from './docs/t02-concepts/into-main-concepts/into-main-concepts.component';

import HomeComponent from './pages/home/home.component';
import { NotFoundComponentComponent } from './pages/error/404/not-found.component';
import { Docs_T03_IntoComponent } from './docs/t03-getting-started/into-main-getting/into-main-getting.component';
// import { Docs_T02_Mod01_Component } from './docs/t02-concepts/mod-01-levels-structure/mod-01-levels-structure.component';
// import { Docs_T02_Mod02_Component } from './docs/t02-concepts/mod-02-support-folders/mod-02-support-folders.component';

export const routes: Routes = [
  { path: '', component: HomeComponent,  
    children: [
      { path: 'introduction', component: Docs_T01_IntoComponent },
      { path: 'concepts', component: Docs_T02_IntoComponent },
      { path: 'getting-started', component: Docs_T03_IntoComponent },
      { path: '', redirectTo: 'introduction', pathMatch: 'full' },
    ]
  },
  { path: 'not-found', component: NotFoundComponentComponent },
  { path: '**', redirectTo: 'not-found', pathMatch: 'full' },
];

@NgModule({
  declarations: [  ],
  imports:[
    BrowserModule,
    RouterModule.forRoot(routes, {
      useHash: true,
    })
  ],
  providers: [
    // {
    //   provide: LocationStrategy,
    //   useClass: HashLocationStrategy
    // }
  ]
})

export class AppRoutingModule { }