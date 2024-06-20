import { Injectable } from '@angular/core';
import { Router, NavigationEnd } from '@angular/router';
import { BehaviorSubject } from 'rxjs';
import { filter } from 'rxjs/operators';

@Injectable({
  providedIn: 'root'
})
export class RouterOutletService {
  private isEmptySubject = new BehaviorSubject<boolean>(true);
  public isEmpty$ = this.isEmptySubject.asObservable();

  constructor(private router: Router) {
    this.router.events.pipe(
      filter(event => event instanceof NavigationEnd)
    ).subscribe(() => {
      this.checkOutletContent();
    });
  }

  checkOutletContent() {
    // Lógica para determinar si el router-outlet tiene contenido
    const hasContent = !!document.querySelector('router-outlet ~ *');
    this.isEmptySubject.next(!hasContent);
  }
}