import { Injectable } from '@angular/core';
import { Router, NavigationEnd } from '@angular/router';
import { BehaviorSubject } from 'rxjs';
import { filter } from 'rxjs/operators';

@Injectable({
  providedIn: 'root',
})
export class RouterOutletService {
  private _isEmpty$ = new BehaviorSubject<boolean>(true);
  isEmpty$ = this._isEmpty$.asObservable();

  constructor(private router: Router) {
    this.router.events.pipe(
      filter(event => event instanceof NavigationEnd)
    ).subscribe(() => {
      const outlet = document.querySelector('router-outlet');
      this._isEmpty$.next(!outlet?.childNodes.length);
    });
  }

  checkOutletContent() {
    const outlet = document.querySelector('router-outlet');
    this._isEmpty$.next(!outlet?.childNodes.length);
  }
}