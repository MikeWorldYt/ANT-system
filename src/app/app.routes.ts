import { BrowserModule, Title } from '@angular/platform-browser';
import { RouterModule, Routes } from '@angular/router';
import { NgModule } from '@angular/core';

// import { HashLocationStrategy, LocationStrategy } from '@angular/common';

import { Docs_T01_IntoComponent } from './docs/t01-starter/into-getting-starter/into-getting-starter.component';
import { Docs_T02_IntoComponent } from './docs/t02-concepts/into-main-concepts/into-main-concepts.component';
import { Docs_T02_Mod01_Component } from './docs/t02-concepts/mod-01-levels-structure/mod-01-levels-structure.component';
import { Docs_T02_Mod02_Component } from './docs/t02-concepts/mod-02-support-folders/mod-02-support-folders.component';
import { Docs_T02_Mod03_Component } from './docs/t02-concepts/mod-03-nomenclature/mod-03-nomenclature.component';
import { Docs_T03_IntoComponent } from './docs/t03-getting-started/into-main-getting/into-main-getting.component';

import HomeComponent from './pages/home/home.component';
import { NotFoundComponentComponent } from './pages/error/404/not-found.component';

export const routes: Routes = [
  { path: '', component: HomeComponent, // HomeComponent
    children: [
      { path: 'introduction', component: Docs_T01_IntoComponent }, // docs-t01-into-getting-starter
      { path: 'concepts', component: Docs_T02_IntoComponent, // docs-t02-into-main-concepts
        children: [
          { path: 'levels-and-structure', component: Docs_T02_Mod01_Component }, // doc-t02-mod-01-levels-structure
          { path: 'support-folders', component: Docs_T02_Mod02_Component }, // doc-t02-mod-02-support-folders
          { path: 'files-nomenclature', component: Docs_T02_Mod03_Component }, // doc-t02-mod-03-nomenclature
          { path: '', redirectTo: 'levels-and-structure', pathMatch: 'full' },
        ] 
      },
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