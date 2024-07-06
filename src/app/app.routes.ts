import { BrowserModule, Title } from '@angular/platform-browser';
import { RouterModule, Routes } from '@angular/router';
import { NgModule } from '@angular/core';
// import { HashLocationStrategy, LocationStrategy } from '@angular/common';

import { Docs_T01_IntoComponent } from './docs/t01-starter/into-getting-starter/into-getting-starter.component';
import { Docs_T02_IntoComponent } from './docs/t02-concepts/into-main-concepts/into-main-concepts.component';

import { AppComponent } from './app.component';
import HomeComponent from './pages/home/home.component';
import { NotFoundComponentComponent } from './pages/error/404/not-found.component';

export const routes: Routes = [
  {  path: '', component: HomeComponent,  
    children: [
      { path: '', redirectTo: 'docs/introduction', pathMatch: 'full' },
      { path: 'docs', redirectTo: 'docs/introduction', pathMatch: 'full' },
      { path: 'docs/introduction', component: Docs_T01_IntoComponent },
      { path: 'docs/concepts', component: Docs_T02_IntoComponent },
    ]
  },
  { path: 'not-found', component: NotFoundComponentComponent },
  { path: '**', component: NotFoundComponentComponent,  pathMatch: 'full'  }
];

@NgModule({
  declarations: [  ],
  imports:[
    BrowserModule,
    RouterModule.forRoot(routes, {
      useHash: true
    })
  ],
  providers: [
    // {
    //   provide: LocationStrategy,
    //   useClass: HashLocationStrategy
    // }
  ],
  bootstrap: [AppComponent]
})

export class AppRoutingModule { }